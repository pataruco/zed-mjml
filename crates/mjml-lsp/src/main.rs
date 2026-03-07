// lsp-types' Uri has interior mutability (Cell) but is safe to use as a HashMap key
// since its Hash/Eq implementation is based on the string value only.
#![allow(clippy::mutable_key_type)]

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

/// Parses the text with mrml and converts any errors into LSP diagnostics.
fn validate_mjml(text: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    match mrml::parse(text) {
        Ok(output) => {
            // Also report warnings (e.g., unexpected attributes) as diagnostics.
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
mod tests {
    use super::*;

    #[test]
    fn test_byte_offset_to_position_start() {
        let text = "hello\nworld";
        let pos = byte_offset_to_position(text, 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_offset_to_position_same_line() {
        let text = "hello\nworld";
        let pos = byte_offset_to_position(text, 3);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3);
    }

    #[test]
    fn test_byte_offset_to_position_second_line() {
        let text = "hello\nworld";
        let pos = byte_offset_to_position(text, 6);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_offset_to_position_second_line_offset() {
        let text = "hello\nworld";
        let pos = byte_offset_to_position(text, 9);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 3);
    }

    #[test]
    fn test_byte_offset_to_position_end_of_text() {
        let text = "hello\nworld";
        let pos = byte_offset_to_position(text, 11);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 5);
    }

    #[test]
    fn test_byte_offset_to_position_beyond_text() {
        let text = "hello";
        let pos = byte_offset_to_position(text, 100);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 5);
    }

    #[test]
    fn test_byte_offset_to_position_empty_text() {
        let text = "";
        let pos = byte_offset_to_position(text, 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_offset_to_position_multiple_lines() {
        let text = "line1\nline2\nline3";
        let pos = byte_offset_to_position(text, 12);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_offset_to_position_utf16_bmp() {
        // e-acute (U+00E9) is 2 bytes in UTF-8, 1 code unit in UTF-16
        let text = "caf\u{00E9}";
        let pos = byte_offset_to_position(text, 5); // after the e-acute
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 4);
    }

    #[test]
    fn test_byte_offset_to_position_utf16_supplementary() {
        // U+1F600 (grinning face) is 4 bytes in UTF-8, 2 code units in UTF-16
        let text = "a\u{1F600}b";
        let pos = byte_offset_to_position(text, 5); // byte offset of 'b'
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3); // 'a'=1 + emoji=2
    }

    #[test]
    fn test_validate_mjml_valid_document() {
        let text = "<mjml><mj-head /><mj-body /></mjml>";
        let diagnostics = validate_mjml(text);
        assert!(diagnostics.is_empty(), "valid MJML should produce no diagnostics");
    }

    #[test]
    fn test_validate_mjml_empty_string() {
        let text = "";
        let diagnostics = validate_mjml(text);
        assert!(!diagnostics.is_empty(), "empty input should produce a diagnostic");
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostics[0].message, "Missing `<mjml>` root element");
    }

    #[test]
    fn test_validate_mjml_unknown_element_is_accepted() {
        // mrml is permissive and accepts unknown elements without error.
        let text = "<mjml><mj-body><mj-section><invalid-tag /></mj-section></mj-body></mjml>";
        let diagnostics = validate_mjml(text);
        assert!(diagnostics.is_empty(), "mrml accepts unknown elements without error");
    }

    #[test]
    fn test_validate_mjml_unclosed_tag() {
        // Unclosed tags produce a parse error.
        let text = "<mjml><mj-body><mj-section>";
        let diagnostics = validate_mjml(text);
        assert!(!diagnostics.is_empty(), "unclosed tag should produce a diagnostic");
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostics[0].source, Some("mjml".to_string()));
        assert!(
            diagnostics[0].message.contains("end of file")
                || diagnostics[0].message.contains("Unexpected"),
            "message should be human-friendly, got: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn test_validate_mjml_malformed_xml() {
        let text = "<mjml><mj-body>";
        let diagnostics = validate_mjml(text);
        assert!(!diagnostics.is_empty(), "malformed XML should produce a diagnostic");
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn test_validate_mjml_not_mjml() {
        let text = "<html><body>Hello</body></html>";
        let diagnostics = validate_mjml(text);
        assert!(!diagnostics.is_empty(), "non-MJML HTML should produce a diagnostic");
        assert!(
            diagnostics[0].message.contains("html"),
            "message should show the unexpected element, got: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn test_validate_mjml_parser_error_has_position() {
        // Invalid XML on the second line triggers a ParserError with position info.
        let text = "<mjml>\n<<<";
        let diagnostics = validate_mjml(text);
        assert!(!diagnostics.is_empty(), "invalid XML should produce a diagnostic");
        let diag = &diagnostics[0];
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        assert!(
            diag.message.starts_with("Syntax error:"),
            "ParserError should have 'Syntax error:' prefix, got: {}",
            diag.message
        );
        // htmlparser reports this error at row 2, col 1 (1-based),
        // which maps to LSP Position { line: 1, character: 0 }.
        assert_eq!(
            diag.range.start.line, 1,
            "error should be on the second line, got {:?}",
            diag.range
        );
    }

    #[test]
    fn test_span_to_range() {
        let text = "hello\nworld\nfoo";
        let range = span_to_range(text, 6, 11);
        assert_eq!(range.start.line, 1);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 1);
        assert_eq!(range.end.character, 5);
    }

    #[test]
    fn test_snippet_at_normal() {
        let text = "hello world";
        assert_eq!(snippet_at(text, 0, 5), "hello");
    }

    #[test]
    fn test_snippet_at_trims_whitespace() {
        let text = "  hello  ";
        assert_eq!(snippet_at(text, 0, 9), "hello");
    }

    #[test]
    fn test_snippet_at_truncates_long_text() {
        let text = "a".repeat(100);
        let result = snippet_at(&text, 0, 100);
        assert!(result.len() <= 63); // 57 chars + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_snippet_at_out_of_bounds() {
        let text = "short";
        assert_eq!(snippet_at(text, 0, 100), "short");
    }

    #[test]
    fn test_snippet_at_empty_span() {
        let text = "hello";
        assert_eq!(snippet_at(text, 3, 3), "");
    }

    #[test]
    fn test_error_message_unexpected_element_shows_tag() {
        // <html> is not a valid MJML root element, mrml reports UnexpectedElement
        let text = "<html><body>Hello</body></html>";
        let diagnostics = validate_mjml(text);
        assert!(!diagnostics.is_empty());
        // The message should contain the offending text, not just byte offsets
        assert!(
            !diagnostics[0].message.contains("at position"),
            "message should not contain raw byte offsets, got: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn test_error_message_no_root_node() {
        let text = "";
        let diagnostics = validate_mjml(text);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].message, "Missing `<mjml>` root element");
    }

    #[test]
    fn test_error_message_end_of_stream() {
        let text = "<mjml><mj-body>";
        let diagnostics = validate_mjml(text);
        assert!(!diagnostics.is_empty());
        // EndOfStream should produce a helpful "unclosed tags" message
        assert!(
            diagnostics[0].message.contains("unclosed tags")
                || diagnostics[0].message.contains("Unexpected"),
            "EndOfStream message should mention unclosed tags, got: {}",
            diagnostics[0].message
        );
    }

    #[test]
    fn test_find_parent_tag_simple() {
        let text = "<mj-image src=\"x\">some text";
        assert_eq!(find_parent_tag(text, 18), Some("mj-image".to_string()));
    }

    #[test]
    fn test_find_parent_tag_skips_closing_tag() {
        let text = "<mj-section></mj-section><mj-image>text";
        assert_eq!(find_parent_tag(text, 34), Some("mj-image".to_string()));
    }

    #[test]
    fn test_find_parent_tag_skips_comment() {
        let text = "<mj-column><!-- comment --><mj-image>text";
        assert_eq!(find_parent_tag(text, 36), Some("mj-image".to_string()));
    }

    #[test]
    fn test_find_parent_tag_none_when_no_tag() {
        let text = "just plain text";
        assert_eq!(find_parent_tag(text, 5), None);
    }

    #[test]
    fn test_error_message_unexpected_token_shows_parent() {
        // mj-image should not contain text children — mrml reports UnexpectedToken
        let text = "<mjml><mj-body><mj-section><mj-column><mj-image src=\"x\">bad text</mj-image></mj-column></mj-section></mj-body></mjml>";
        let diagnostics = validate_mjml(text);
        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics[0].message.contains("mj-image"),
            "message should mention the parent element, got: {}",
            diagnostics[0].message
        );
    }
}
