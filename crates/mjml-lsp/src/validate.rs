use std::fmt::Write;

use crate::rules::{self, KNOWN_TAGS};
use crate::scanner::TagInfo;

/// Severity levels for lint diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// An automatic fix for a lint diagnostic.
#[derive(Debug, Clone)]
pub struct LintFix {
    pub title: String,
    /// Byte-span replacements to apply. A zero-width span inserts text.
    pub edits: Vec<((usize, usize), String)>,
}

/// A lint diagnostic produced by the tag validator.
#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    /// Byte range in the source text to underline.
    pub span: (usize, usize),
    pub severity: Severity,
    pub message: String,
    pub fix: Option<LintFix>,
}

/// Validates a list of scanned tags against MJML rules.
/// Returns all violations found (does not stop at the first).
pub fn validate_tags(_text: &str, tags: &[TagInfo]) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();

    let mut head_count = 0u32;
    let mut body_count = 0u32;

    for tag in tags {
        // Rule 4: Singleton enforcement
        if tag.name == "mj-head" {
            head_count += 1;
            if head_count > 1 {
                diagnostics.push(LintDiagnostic {
                    span: tag.tag_span,
                    severity: Severity::Error,
                    message: "Duplicate <mj-head> — only one <mj-head> is allowed per document"
                        .to_string(),
                    fix: None,
                });
            }
        }
        if tag.name == "mj-body" {
            body_count += 1;
            if body_count > 1 {
                diagnostics.push(LintDiagnostic {
                    span: tag.tag_span,
                    severity: Severity::Error,
                    message: "Duplicate <mj-body> — only one <mj-body> is allowed per document"
                        .to_string(),
                    fix: None,
                });
            }
        }

        // Only validate mj-* tags (non-mj tags are valid HTML content)
        if !tag.name.starts_with("mj-") && tag.name != "mjml" {
            continue;
        }

        // Rule 3: Unknown mj-* tag
        if tag.name.starts_with("mj-") && !KNOWN_TAGS.contains(tag.name.as_str()) {
            let mut msg = format!("Unknown MJML element <{}>", tag.name);
            let fix = rules::suggest_tag(&tag.name).map(|suggestion| {
                let _ = write!(msg, " — did you mean <{suggestion}>?");
                let name_start = tag.tag_span.0 + 1;
                let name_end = name_start + tag.name.len();
                LintFix {
                    title: format!("Replace with <{suggestion}>"),
                    edits: vec![((name_start, name_end), suggestion.to_string())],
                }
            });
            diagnostics.push(LintDiagnostic {
                span: tag.tag_span,
                severity: Severity::Warning,
                message: msg,
                fix,
            });
            continue; // skip nesting/attr checks for unknown tags
        }

        // Rule 1: Nesting
        if let Some(allowed) = rules::allowed_parents(&tag.name) {
            let actual_parent = tag.parent_idx.map(|i| tags[i].name.as_str());
            let is_valid = actual_parent.is_some_and(|parent_name| allowed.contains(&parent_name));
            // "mjml" has no parent entry in allowed_parents, so skip it
            if !is_valid && tag.name != "mjml" {
                let parent_display = actual_parent.unwrap_or("document root");
                let expected = allowed
                    .iter()
                    .map(|p| format!("<{p}>"))
                    .collect::<Vec<_>>()
                    .join(" or ");
                diagnostics.push(LintDiagnostic {
                    span: tag.tag_span,
                    severity: Severity::Error,
                    message: format!(
                        "<{}> must be inside {}, but found inside <{}>",
                        tag.name, expected, parent_display
                    ),
                    fix: None,
                });
            }
        }

        // Rule 2: Required attributes
        let required = rules::required_attributes(&tag.name);
        if !required.is_empty() {
            let present: Vec<&str> = tag.attributes.iter().map(|a| a.name.as_str()).collect();
            for attr_name in required {
                if !present.contains(&attr_name) {
                    let insert_at = tag.tag_span.0 + 1 + tag.name.len();
                    diagnostics.push(LintDiagnostic {
                        span: tag.tag_span,
                        severity: Severity::Warning,
                        message: format!(
                            "<{}> is missing required attribute \"{}\"",
                            tag.name, attr_name
                        ),
                        fix: Some(LintFix {
                            title: format!("Add required attribute \"{attr_name}\""),
                            edits: vec![((insert_at, insert_at), format!(" {attr_name}=\"\""))],
                        }),
                    });
                }
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::scan_tags;

    fn validate(text: &str) -> Vec<LintDiagnostic> {
        let tags = scan_tags(text);
        validate_tags(text, &tags)
    }

    #[test]
    fn test_valid_document_no_errors() {
        let diags = validate("<mjml><mj-head><mj-title>Hi</mj-title></mj-head><mj-body><mj-section><mj-column><mj-text>Hello</mj-text></mj-column></mj-section></mj-body></mjml>");
        assert!(
            diags.is_empty(),
            "valid document should produce no diagnostics, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_nesting_mj_text_in_section() {
        let diags = validate(
            "<mjml><mj-body><mj-section><mj-text>Bad</mj-text></mj-section></mj-body></mjml>",
        );
        assert!(!diags.is_empty());
        let d = &diags[0];
        assert_eq!(d.severity, Severity::Error);
        assert!(d.message.contains("<mj-text>"), "msg: {}", d.message);
        assert!(
            d.message.contains("<mj-column>") || d.message.contains("<mj-hero>"),
            "msg: {}",
            d.message
        );
        assert!(d.message.contains("<mj-section>"), "msg: {}", d.message);
    }

    #[test]
    fn test_nesting_mj_section_in_body_ok() {
        let diags =
            validate("<mjml><mj-body><mj-section><mj-column /></mj-section></mj-body></mjml>");
        // mj-section in mj-body is valid
        let nesting_errors: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("must be inside"))
            .collect();
        assert!(nesting_errors.is_empty());
    }

    #[test]
    fn test_nesting_mj_column_outside_section() {
        let diags = validate(
            "<mjml><mj-body><mj-column><mj-text>Bad</mj-text></mj-column></mj-body></mjml>",
        );
        assert!(!diags.is_empty());
        assert!(diags[0].message.contains("<mj-column>"));
    }

    #[test]
    fn test_required_attr_mj_image_missing_src() {
        let diags = validate("<mjml><mj-body><mj-section><mj-column><mj-image alt=\"x\" /></mj-column></mj-section></mj-body></mjml>");
        let attr_diags: Vec<_> = diags.iter().filter(|d| d.message.contains("src")).collect();
        assert!(!attr_diags.is_empty(), "should warn about missing src");
        assert_eq!(attr_diags[0].severity, Severity::Warning);
    }

    #[test]
    fn test_required_attr_mj_image_has_src() {
        let diags = validate("<mjml><mj-body><mj-section><mj-column><mj-image src=\"x.png\" /></mj-column></mj-section></mj-body></mjml>");
        let attr_diags: Vec<_> = diags.iter().filter(|d| d.message.contains("src")).collect();
        assert!(attr_diags.is_empty());
    }

    #[test]
    fn test_unknown_mj_tag() {
        let diags = validate("<mjml><mj-body><mj-section><mj-column><mj-seciton /></mj-column></mj-section></mj-body></mjml>");
        let unknown_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("Unknown"))
            .collect();
        assert!(!unknown_diags.is_empty());
        assert_eq!(unknown_diags[0].severity, Severity::Warning);
        assert!(
            unknown_diags[0].message.contains("mj-section"),
            "should suggest correction, msg: {}",
            unknown_diags[0].message
        );
    }

    #[test]
    fn test_unknown_non_mj_tag_ignored() {
        let diags = validate("<mjml><mj-body><mj-section><mj-column><mj-text><div>ok</div></mj-text></mj-column></mj-section></mj-body></mjml>");
        let unknown_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("Unknown"))
            .collect();
        assert!(
            unknown_diags.is_empty(),
            "non-mj tags should not trigger unknown tag warning"
        );
    }

    #[test]
    fn test_duplicate_mj_body() {
        let diags = validate("<mjml><mj-body></mj-body><mj-body></mj-body></mjml>");
        let dup_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("Duplicate"))
            .collect();
        assert!(!dup_diags.is_empty());
        assert_eq!(dup_diags[0].severity, Severity::Error);
    }

    #[test]
    fn test_duplicate_mj_head() {
        let diags = validate("<mjml><mj-head></mj-head><mj-head></mj-head></mjml>");
        let dup_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("Duplicate"))
            .collect();
        assert!(!dup_diags.is_empty());
    }

    #[test]
    fn test_reports_multiple_errors() {
        // Two nesting violations in one document
        let diags = validate("<mjml><mj-body><mj-section><mj-text>A</mj-text><mj-image src=\"x\" /></mj-section></mj-body></mjml>");
        let nesting: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("must be inside"))
            .collect();
        assert!(
            nesting.len() >= 2,
            "should report both nesting errors, got {}",
            nesting.len()
        );
    }

    #[test]
    fn test_required_attr_mj_font() {
        let diags = validate("<mjml><mj-head><mj-font /></mj-head></mjml>");
        let attr_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("missing required"))
            .collect();
        assert!(
            attr_diags.len() >= 2,
            "mj-font requires name and href, got {} warnings",
            attr_diags.len()
        );
    }

    #[test]
    fn test_unknown_tag_has_replace_fix() {
        let diags = validate("<mjml><mj-body><mj-section><mj-column><mj-seciton /></mj-column></mj-section></mj-body></mjml>");
        let d = diags
            .iter()
            .find(|d| d.message.contains("Unknown"))
            .expect("should report the unknown tag");
        let fix = d.fix.as_ref().expect("unknown tag should carry a fix");
        assert!(fix.title.contains("mj-section"));
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].1, "mj-section");
    }

    #[test]
    fn test_missing_attr_has_insert_fix() {
        let diags = validate("<mjml><mj-body><mj-section><mj-column><mj-image alt=\"x\" /></mj-column></mj-section></mj-body></mjml>");
        let d = diags
            .iter()
            .find(|d| d.message.contains("src"))
            .expect("should report the missing src");
        let fix = d
            .fix
            .as_ref()
            .expect("missing attribute should carry a fix");
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].1, " src=\"\"");
    }
}
