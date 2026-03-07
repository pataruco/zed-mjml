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
    // Non-mj-* tags are treated as valid HTML content by the scanner.
    let text = "<mjml><mj-body><mj-section><mj-column><invalid-tag /></mj-column></mj-section></mj-body></mjml>";
    let diagnostics = validate_mjml(text);
    let unknown: Vec<_> = diagnostics.iter().filter(|d| d.message.contains("Unknown")).collect();
    assert!(unknown.is_empty(), "non-mj-* tags should not trigger unknown tag warning");
}

#[test]
fn test_validate_mjml_unclosed_tag() {
    // Unclosed tags produce a parse error.
    let text = "<mjml><mj-body><mj-section>";
    let diagnostics = validate_mjml(text);
    assert!(!diagnostics.is_empty(), "unclosed tag should produce a diagnostic");
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.severity == Some(DiagnosticSeverity::ERROR)).collect();
    assert!(!errors.is_empty());
    assert_eq!(diagnostics.last().unwrap().source, Some("mjml".to_string()));
}

#[test]
fn test_validate_mjml_malformed_xml() {
    let text = "<mjml><mj-body>";
    let diagnostics = validate_mjml(text);
    assert!(!diagnostics.is_empty(), "malformed XML should produce a diagnostic");
}

#[test]
fn test_validate_mjml_not_mjml() {
    let text = "<html><body>Hello</body></html>";
    let diagnostics = validate_mjml(text);
    assert!(!diagnostics.is_empty(), "non-MJML HTML should produce a diagnostic");
    assert!(
        diagnostics.iter().any(|d| d.message.contains("html")),
        "message should show the unexpected element, got: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_validate_mjml_parser_error_has_position() {
    // Invalid XML on the second line triggers a ParserError with position info.
    let text = "<mjml>\n<<<";
    let diagnostics = validate_mjml(text);
    assert!(!diagnostics.is_empty(), "invalid XML should produce a diagnostic");
    let parser_diags: Vec<_> = diagnostics.iter().filter(|d| d.message.starts_with("Syntax error:")).collect();
    assert!(!parser_diags.is_empty(), "should have a syntax error diagnostic");
    assert_eq!(
        parser_diags[0].range.start.line, 1,
        "error should be on the second line, got {:?}",
        parser_diags[0].range
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
        diagnostics.iter().any(|d| !d.message.contains("at position")),
        "message should not contain raw byte offsets"
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
    assert!(
        diagnostics.iter().any(|d|
            d.message.contains("unclosed tags") || d.message.contains("Unexpected")
        ),
        "should mention unclosed tags, got: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
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
        diagnostics.iter().any(|d| d.message.contains("mj-image")),
        "message should mention the parent element, got: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// --- Integration tests for scanner-based validation ---

#[test]
fn test_validate_mjml_nesting_error() {
    let text = "<mjml><mj-body><mj-section><mj-text>Bad</mj-text></mj-section></mj-body></mjml>";
    let diagnostics = validate_mjml(text);
    let nesting: Vec<_> = diagnostics.iter().filter(|d| d.message.contains("must be inside")).collect();
    assert!(!nesting.is_empty(), "should catch nesting violation, got: {:?}", diagnostics);
    assert_eq!(nesting[0].severity, Some(DiagnosticSeverity::ERROR));
}

#[test]
fn test_validate_mjml_missing_required_attr() {
    let text = "<mjml><mj-body><mj-section><mj-column><mj-image alt=\"x\" /></mj-column></mj-section></mj-body></mjml>";
    let diagnostics = validate_mjml(text);
    let missing: Vec<_> = diagnostics.iter().filter(|d| d.message.contains("src")).collect();
    assert!(!missing.is_empty(), "should warn about missing src");
    assert_eq!(missing[0].severity, Some(DiagnosticSeverity::WARNING));
}

#[test]
fn test_validate_mjml_unknown_tag_warning() {
    let text = "<mjml><mj-body><mj-section><mj-column><mj-seciton /></mj-column></mj-section></mj-body></mjml>";
    let diagnostics = validate_mjml(text);
    let unknown: Vec<_> = diagnostics.iter().filter(|d| d.message.contains("Unknown")).collect();
    assert!(!unknown.is_empty(), "should warn about unknown mj-* tag");
    assert!(unknown[0].message.contains("mj-section"), "should suggest correction");
}

#[test]
fn test_validate_mjml_duplicate_body() {
    let text = "<mjml><mj-body></mj-body><mj-body></mj-body></mjml>";
    let diagnostics = validate_mjml(text);
    let dups: Vec<_> = diagnostics.iter().filter(|d| d.message.contains("Duplicate")).collect();
    assert!(!dups.is_empty(), "should catch duplicate mj-body");
}

#[test]
fn test_validate_mjml_multiple_errors_reported() {
    // Two nesting violations — both should be reported
    let text = "<mjml><mj-body><mj-section><mj-text>A</mj-text><mj-image src=\"x\" /></mj-section></mj-body></mjml>";
    let diagnostics = validate_mjml(text);
    let nesting: Vec<_> = diagnostics.iter().filter(|d| d.message.contains("must be inside")).collect();
    assert!(nesting.len() >= 2, "should report all nesting errors, got {}", nesting.len());
}

#[test]
fn test_validate_mjml_valid_document_still_clean() {
    let text = "<mjml><mj-head><mj-title>Hi</mj-title></mj-head><mj-body><mj-section><mj-column><mj-text>Hello</mj-text></mj-column></mj-section></mj-body></mjml>";
    let diagnostics = validate_mjml(text);
    assert!(diagnostics.is_empty(), "valid document should have no diagnostics, got: {:?}", diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>());
}
