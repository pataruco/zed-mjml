// lsp-types' Uri has interior mutability (Cell) but is safe to use as a HashMap key
// since its Hash/Eq implementation is based on the string value only.
#![allow(clippy::mutable_key_type)]

mod rules;
mod scanner;
mod validate;

use std::collections::HashMap;
use std::error::Error;

use lsp_server::{Connection, Message, Notification, Response};
use lsp_types::notification::Notification as _;
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, InitializeParams, Position, PublishDiagnosticsParams, Range,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
                   PublishDiagnostics},
};

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("mjml-lsp: starting");

    let (connection, io_threads) = Connection::stdio();

    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
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
    let _init_params: InitializeParams = serde_json::from_value(params)?;
    let mut documents: HashMap<Uri, String> = HashMap::new();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
                // We don't handle any requests beyond shutdown for now.
                // Respond with MethodNotFound for unknown requests.
                let resp = Response::new_err(
                    req.id,
                    lsp_server::ErrorCode::MethodNotFound as i32,
                    "method not supported".to_string(),
                );
                connection.sender.send(Message::Response(resp))?;
            }
            Message::Notification(notification) => {
                handle_notification(connection, &notification, &mut documents)?;
            }
            Message::Response(_) => {
                // We don't send requests, so we don't expect responses.
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
        diagnostics.push(Diagnostic {
            range,
            severity: Some(severity),
            code: None,
            code_description: None,
            source: Some("mjml".to_string()),
            message: lint.message,
            related_information: None,
            tags: None,
            data: None,
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
    let s = text
        .get(start..end.min(text.len()))
        .unwrap_or("")
        .trim();
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
fn error_to_range_and_message(
    text: &str,
    err: &mrml::prelude::parser::Error,
) -> (Range, String) {
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
                (false, None) => format!(
                    "Unexpected token: `{snippet}` is not valid here"
                ),
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
        Error::MissingAttribute {
            name, position, ..
        } => {
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
            let lsp_pos = Position::new(
                pos.row.saturating_sub(1),
                pos.col.saturating_sub(1),
            );
            let range = Range::new(lsp_pos, lsp_pos);
            let message = format!("Syntax error: {source}");
            (range, message)
        }
        Error::EndOfStream { .. } => {
            let range = Range::new(Position::new(0, 0), Position::new(0, 0));
            (range, "Unexpected end of file — check for unclosed tags".to_string())
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
            character += ch.len_utf16() as u32;
        }
    }

    Position::new(line, character)
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
    let notification = Notification::new(
        PublishDiagnostics::METHOD.to_string(),
        params,
    );
    connection
        .sender
        .send(Message::Notification(notification))?;
    Ok(())
}

#[cfg(test)]
mod tests;
