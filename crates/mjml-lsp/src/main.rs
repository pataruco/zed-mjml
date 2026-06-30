// lsp-types' Uri has interior mutability (Cell) but is safe to use as a HashMap key
// since its Hash/Eq implementation is based on the string value only.
#![allow(clippy::mutable_key_type)]

mod code_action;
mod completion;
mod hover;
mod preview;
mod rules;
mod scanner;
mod validate;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use lsp_server::{Connection, Message, Notification, Request as ServerRequest, Response};
use lsp_types::notification::Notification as _;
use lsp_types::request::{CodeActionRequest, Completion, HoverRequest, Request as _};
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, PublishDiagnostics,
    },
    CodeActionParams, CodeActionProviderCapability, CompletionOptions, Diagnostic,
    DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, HoverProviderCapability, InitializeParams, Position,
    PublishDiagnosticsParams, Range, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, Uri,
};

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("mjml-lsp: starting");

    let (connection, io_threads) = Connection::stdio();

    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![
                "<".to_string(),
                " ".to_string(),
                "\"".to_string(),
                "'".to_string(),
                "=".to_string(),
            ]),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        ..Default::default()
    };

    let server_capabilities = serde_json::to_value(capabilities)?;
    let init_params = connection.initialize(server_capabilities)?;

    main_loop(&connection, init_params)?;
    io_threads.join()?;

    eprintln!("mjml-lsp: shutdown complete");
    Ok(())
}

fn main_loop(
    connection: &Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let init_params: InitializeParams = serde_json::from_value(params)?;
    let preview_enabled = preview_enabled(&init_params);
    let mut documents: HashMap<Uri, String> = HashMap::new();
    // Counter for preview marker nonces and applyEdit request ids, plus the set
    // of nonces already fired. The set makes the applyEdit strip's didChange a
    // no-op so the preview fires exactly once per selection.
    let next_id = AtomicU64::new(1);
    let handled: Mutex<HashSet<String>> = Mutex::new(HashSet::new());

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
                let resp = match req.method.as_str() {
                    Completion::METHOD => completion::handle(&req, &documents),
                    HoverRequest::METHOD => hover::handle(&req, &documents),
                    CodeActionRequest::METHOD => {
                        let mut response = code_action::handle(&req);
                        if preview_enabled {
                            append_preview_action(&mut response, &req, &documents, &next_id);
                        }
                        response
                    }
                    _ => Response::new_err(
                        req.id.clone(),
                        lsp_server::ErrorCode::MethodNotFound as i32,
                        "method not supported".to_string(),
                    ),
                };
                connection.sender.send(Message::Response(resp))?;
            }
            Message::Notification(notification) => {
                handle_notification(
                    connection,
                    &notification,
                    &mut documents,
                    preview_enabled,
                    &next_id,
                    &handled,
                )?;
            }
            Message::Response(_) => {
                // applyEdit acknowledgements; nothing to do with them.
            }
        }
    }

    Ok(())
}

/// Dispatches incoming notifications to the appropriate handler.
fn handle_notification(
    connection: &Connection,
    notification: &Notification,
    documents: &mut HashMap<Uri, String>,
    preview_enabled: bool,
    next_id: &AtomicU64,
    handled: &Mutex<HashSet<String>>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    match notification.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params: DidOpenTextDocumentParams =
                serde_json::from_value(notification.params.clone())?;
            let uri = params.text_document.uri;
            let text = params.text_document.text;
            documents.insert(uri.clone(), text.clone());
            validate_and_publish(connection, &uri, &text)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams =
                serde_json::from_value(notification.params.clone())?;
            // With TextDocumentSyncKind::FULL, the first content change contains
            // the entire new document text.
            if let Some(change) = params.content_changes.into_iter().next() {
                let uri = params.text_document.uri;
                documents.insert(uri.clone(), change.text.clone());
                validate_and_publish(connection, &uri, &change.text)?;
                if preview_enabled {
                    handle_preview_marker(connection, &uri, &change.text, next_id, handled)?;
                }
            }
        }
        DidCloseTextDocument::METHOD => {
            let params: DidCloseTextDocumentParams =
                serde_json::from_value(notification.params.clone())?;
            let uri = params.text_document.uri;
            documents.remove(&uri);
            // Clear diagnostics for the closed document.
            publish_diagnostics(connection, &uri, vec![])?;
        }
        _ => {
            // Ignore unknown notifications (per LSP spec).
        }
    }

    Ok(())
}

/// Reads `mjml.preview.enabled` from the client's initialization options,
/// defaulting to enabled when unset so the feature works out of the box.
fn preview_enabled(init: &InitializeParams) -> bool {
    init.initialization_options
        .as_ref()
        .and_then(|options| options.get("mjml"))
        .and_then(|mjml| mjml.get("preview"))
        .and_then(|preview| preview.get("enabled"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

/// Derives a file stem from a document URI to name its temp preview file.
fn preview_stem(uri: &Uri) -> String {
    let basename = uri.as_str().rsplit('/').next().unwrap_or("mjml-preview");
    match basename.rsplit_once('.') {
        Some((stem, _)) => stem.to_string(),
        None => basename.to_string(),
    }
}

/// Appends the "Open Preview in Browser" code action to a code-action response.
/// The action carries no `command` (Zed would ignore it) and inserts a nonce
/// marker at EOF; the selection is detected later in didChange.
fn append_preview_action(
    response: &mut Response,
    request: &ServerRequest,
    documents: &HashMap<Uri, String>,
    next_id: &AtomicU64,
) {
    let Ok(params) = serde_json::from_value::<CodeActionParams>(request.params.clone()) else {
        return;
    };
    let uri = params.text_document.uri;
    let nonce = next_id.fetch_add(1, Ordering::SeqCst);
    let at = documents.get(&uri).map_or_else(
        || Position::new(0, 0),
        |text| byte_offset_to_position(text, text.len()),
    );
    let action = preview::build_preview_action(uri, nonce, at);
    if let Some(arr) = response
        .result
        .as_mut()
        .and_then(serde_json::Value::as_array_mut)
    {
        if let Ok(value) = serde_json::to_value(action) {
            arr.push(value);
        }
    }
}

/// On a freshly-applied marker: render the document, write the temp HTML, open
/// the browser, record the nonce, then ask the client to strip the marker via
/// `workspace/applyEdit`. Already-handled or absent markers are no-ops.
fn handle_preview_marker(
    connection: &Connection,
    uri: &Uri,
    text: &str,
    next_id: &AtomicU64,
    handled: &Mutex<HashSet<String>>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let decision = preview::classify_marker(text, &handled.lock().expect("handled mutex poisoned"));
    let preview::MarkerDecision::Fire { nonce, start, end } = decision else {
        return Ok(());
    };
    handled
        .lock()
        .expect("handled mutex poisoned")
        .insert(nonce);

    let html = preview::render_to_html(text)
        .unwrap_or_else(|err| preview::error_page_html(&err.to_string()));
    let stem = preview_stem(uri);
    match preview::write_temp_html(&html, &stem) {
        Ok(path) => {
            if let Err(err) = preview::open_in_browser(&path) {
                eprintln!("mjml-lsp: preview browser open failed: {err}");
            }
        }
        Err(err) => eprintln!("mjml-lsp: preview file write failed: {err}"),
    }

    let id = i32::try_from(next_id.fetch_add(1, Ordering::SeqCst)).unwrap_or(i32::MAX);
    let range = span_to_range(text, start, end);
    let request = preview::strip_marker_request(id, uri.clone(), range);
    connection.sender.send(Message::Request(request))?;
    Ok(())
}

/// Validates the MJML document and publishes diagnostics to the client.
fn validate_and_publish(
    connection: &Connection,
    uri: &Uri,
    text: &str,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let diagnostics = validate_mjml(text);
    publish_diagnostics(connection, uri, diagnostics)?;
    Ok(())
}

/// Validates the MJML document using both the tag scanner (semantic rules)
/// and mrml parser (structural rules). Returns all diagnostics.
fn validate_mjml(text: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Pass 1: Tag scanner + MJML semantic validation
    let tags = scanner::scan_tags(text);
    let lint_diagnostics = validate::validate_tags(text, &tags);
    for lint in lint_diagnostics {
        let range = span_to_range(text, lint.span.0, lint.span.1);
        let severity = match lint.severity {
            validate::Severity::Error => DiagnosticSeverity::ERROR,
            validate::Severity::Warning => DiagnosticSeverity::WARNING,
        };
        let data = lint.fix.and_then(|fix| {
            let edits = fix
                .edits
                .into_iter()
                .map(|(byte_span, new_text)| code_action::FixEdit {
                    range: span_to_range(text, byte_span.0, byte_span.1),
                    new_text,
                })
                .collect();
            serde_json::to_value(code_action::DiagnosticFix {
                title: fix.title,
                edits,
            })
            .ok()
        });
        diagnostics.push(Diagnostic {
            range,
            severity: Some(severity),
            code: None,
            code_description: None,
            source: Some("mjml".to_string()),
            message: lint.message,
            related_information: None,
            tags: None,
            data,
        });
    }

    // Pass 2: mrml structural validation
    match mrml::parse(text) {
        Ok(output) => {
            for warning in output.warnings {
                let range = span_to_range(text, warning.span.start, warning.span.end);
                diagnostics.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: None,
                    code_description: None,
                    source: Some("mjml".to_string()),
                    message: warning.to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }
        Err(err) => {
            let (range, message) = error_to_range_and_message(text, &err);
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("mjml".to_string()),
                message,
                related_information: None,
                tags: None,
                data: None,
            });
        }
    }

    diagnostics
}

/// Extracts the source text at a byte span, trimmed and truncated for display.
fn snippet_at(text: &str, start: usize, end: usize) -> String {
    let s = text.get(start..end.min(text.len())).unwrap_or("").trim();
    if s.len() > 60 {
        format!("{}...", &s[..57])
    } else {
        s.to_string()
    }
}

/// Scans backwards from `byte_offset` in `text` to find the nearest opening tag name.
/// Returns the tag name (e.g. "mj-image") or None if not found.
fn find_parent_tag(text: &str, byte_offset: usize) -> Option<String> {
    let before = text.get(..byte_offset.min(text.len()))?;
    // Find the last '<' that starts an opening tag (not '</' or '<!--')
    let mut search_from = before.len();
    loop {
        let idx = before[..search_from].rfind('<')?;
        let after_bracket = &before[idx + 1..];
        // Skip closing tags and comments
        if after_bracket.starts_with('/') || after_bracket.starts_with('!') {
            if idx == 0 {
                return None;
            }
            search_from = idx;
            continue;
        }
        // Extract the tag name (sequence of alphanumeric + hyphen chars)
        let tag_name: String = after_bracket
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-')
            .collect();
        if tag_name.is_empty() {
            if idx == 0 {
                return None;
            }
            search_from = idx;
            continue;
        }
        return Some(tag_name);
    }
}

/// Extracts position information from a mrml parser error and returns a
/// Range and human-friendly error message suitable for an LSP diagnostic.
fn error_to_range_and_message(text: &str, err: &mrml::prelude::parser::Error) -> (Range, String) {
    use mrml::prelude::parser::Error;

    match err {
        Error::UnexpectedElement { position, .. } => {
            let range = span_to_range(text, position.start, position.end);
            let snippet = snippet_at(text, position.start, position.end);
            let parent = find_parent_tag(text, position.start);
            let message = match (snippet.is_empty(), parent) {
                (true, _) => "Unexpected element".to_string(),
                (false, Some(tag)) => format!(
                    "Unexpected element `{snippet}` inside `<{tag}>` — `<{tag}>` cannot contain this element"
                ),
                (false, None) => format!(
                    "Unexpected element: `{snippet}` is not valid here"
                ),
            };
            (range, message)
        }
        Error::UnexpectedToken { position, .. } => {
            let range = span_to_range(text, position.start, position.end);
            let snippet = snippet_at(text, position.start, position.end);
            let parent = find_parent_tag(text, position.start);
            let message = match (snippet.is_empty(), parent) {
                (true, _) => "Unexpected token".to_string(),
                (false, Some(tag)) => format!(
                    "Unexpected content inside `<{tag}>` — `<{tag}>` cannot contain `{snippet}`"
                ),
                (false, None) => format!("Unexpected token: `{snippet}` is not valid here"),
            };
            (range, message)
        }
        Error::InvalidAttribute { position, .. } => {
            let range = span_to_range(text, position.start, position.end);
            let snippet = snippet_at(text, position.start, position.end);
            let message = if snippet.is_empty() {
                "Invalid attribute".to_string()
            } else {
                format!("Invalid attribute in `{snippet}`")
            };
            (range, message)
        }
        Error::InvalidFormat { position, .. } => {
            let range = span_to_range(text, position.start, position.end);
            let snippet = snippet_at(text, position.start, position.end);
            let message = if snippet.is_empty() {
                "Invalid format".to_string()
            } else {
                format!("Invalid format in `{snippet}`")
            };
            (range, message)
        }
        Error::MissingAttribute { name, position, .. } => {
            let range = span_to_range(text, position.start, position.end);
            let snippet = snippet_at(text, position.start, position.end);
            let message = if snippet.is_empty() {
                format!("Missing required attribute `{name}`")
            } else {
                format!("Missing required attribute `{name}` on `{snippet}`")
            };
            (range, message)
        }
        Error::IncludeLoaderError {
            position, source, ..
        } => {
            let range = span_to_range(text, position.start, position.end);
            let message = format!("Failed to load include `{}`: {source}", source.path);
            (range, message)
        }
        // htmlparser errors carry a TextPos with 1-based row/col.
        Error::ParserError { source, .. } => {
            let pos = source.pos();
            // htmlparser TextPos is 1-based; LSP Position is 0-based.
            let lsp_pos = Position::new(pos.row.saturating_sub(1), pos.col.saturating_sub(1));
            let range = Range::new(lsp_pos, lsp_pos);
            let message = format!("Syntax error: {source}");
            (range, message)
        }
        Error::EndOfStream { .. } => {
            let range = Range::new(Position::new(0, 0), Position::new(0, 0));
            (
                range,
                "Unexpected end of file — check for unclosed tags".to_string(),
            )
        }
        Error::SizeLimit { .. } => {
            let range = Range::new(Position::new(0, 0), Position::new(0, 0));
            (range, "Document exceeds the maximum size limit".to_string())
        }
        Error::NoRootNode => {
            let range = Range::new(Position::new(0, 0), Position::new(0, 0));
            (range, "Missing `<mjml>` root element".to_string())
        }
    }
}

/// Converts a byte-offset span (start..end) in the source text into an LSP Range
/// with 0-based line and character (UTF-16 code unit offset).
fn span_to_range(text: &str, start: usize, end: usize) -> Range {
    let start_pos = byte_offset_to_position(text, start);
    let end_pos = byte_offset_to_position(text, end);
    Range::new(start_pos, end_pos)
}

/// Converts a byte offset in the source text to an LSP Position (0-based line
/// and character). The character offset counts UTF-16 code units, matching
/// the LSP specification's default position encoding.
fn byte_offset_to_position(text: &str, byte_offset: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;

    for (i, ch) in text.char_indices() {
        if i >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            // Count UTF-16 code units: characters in the Basic Multilingual Plane
            // take 1 code unit, supplementary characters take 2 (surrogate pair).
            #[expect(clippy::cast_possible_truncation)]
            {
                character += ch.len_utf16() as u32;
            }
        }
    }

    Position::new(line, character)
}

/// Converts an LSP Position (0-based line and UTF-16 character offset) into a
/// byte offset in the source text. Clamps to line and document bounds.
fn position_to_offset(text: &str, pos: Position) -> usize {
    let mut line = 0u32;
    let mut character = 0u32;

    for (i, ch) in text.char_indices() {
        if line == pos.line && character >= pos.character {
            return i;
        }
        if ch == '\n' {
            if line == pos.line {
                // Target character is past the end of this line; clamp to the newline.
                return i;
            }
            line += 1;
            character = 0;
        } else {
            #[expect(clippy::cast_possible_truncation)]
            {
                character += ch.len_utf16() as u32;
            }
        }
    }

    text.len()
}

/// Sends a publishDiagnostics notification to the client.
fn publish_diagnostics(
    connection: &Connection,
    uri: &Uri,
    diagnostics: Vec<Diagnostic>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };
    let notification = Notification::new(PublishDiagnostics::METHOD.to_string(), params);
    connection
        .sender
        .send(Message::Notification(notification))?;
    Ok(())
}

#[cfg(test)]
mod tests;
