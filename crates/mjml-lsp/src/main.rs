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

/// Extracts position information from a mrml parser error and returns a
/// Range and error message suitable for an LSP diagnostic.
fn error_to_range_and_message(
    text: &str,
    err: &mrml::prelude::parser::Error,
) -> (Range, String) {
    use mrml::prelude::parser::Error;

    let message = err.to_string();

    match err {
        // Variants that carry a Span (byte offsets into the source text).
        Error::UnexpectedElement { position, .. }
        | Error::UnexpectedToken { position, .. }
        | Error::InvalidAttribute { position, .. }
        | Error::InvalidFormat { position, .. } => {
            let range = span_to_range(text, position.start, position.end);
            (range, message)
        }
        Error::MissingAttribute { position, .. }
        | Error::IncludeLoaderError { position, .. } => {
            let range = span_to_range(text, position.start, position.end);
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
            (range, message)
        }
        // Variants without position information: report at the start of the document.
        Error::EndOfStream { .. } | Error::SizeLimit { .. } | Error::NoRootNode => {
            let range = Range::new(Position::new(0, 0), Position::new(0, 0));
            (range, message)
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
    }

    #[test]
    fn test_validate_mjml_parser_error_has_position() {
        // Invalid XML on the second line triggers a ParserError with position info.
        let text = "<mjml>\n<<<";
        let diagnostics = validate_mjml(text);
        assert!(!diagnostics.is_empty(), "invalid XML should produce a diagnostic");
        let diag = &diagnostics[0];
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
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
}
