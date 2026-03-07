# MJML Full Validation Rules — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a pre-parse tag scanner that validates MJML nesting rules, required attributes, unknown tags, and singleton constraints — reporting ALL violations with meaningful error messages.

**Architecture:** A new `scanner.rs` module scans source text into `Vec<TagInfo>` with byte positions. A new `rules.rs` defines the MJML spec (allowed parents, required attrs, known tags). A new `validate.rs` walks the tags and produces `Vec<Diagnostic>`. The existing `main.rs` calls both passes (scanner + mrml) and merges diagnostics.

**Tech Stack:** Rust, lsp-types 0.97, mrml 5

**Reference:** Design doc at `docs/plans/2026-03-07-mjml-validation-rules-design.md`

---

### Task 1: Create `rules.rs` — MJML specification data

**Files:**
- Create: `crates/mjml-lsp/src/rules.rs`
- Modify: `crates/mjml-lsp/src/main.rs` (add `mod rules;`)

**Step 1: Write the test**

Add to bottom of `rules.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_tags_contains_all_mjml_elements() {
        assert!(KNOWN_TAGS.contains("mjml"));
        assert!(KNOWN_TAGS.contains("mj-body"));
        assert!(KNOWN_TAGS.contains("mj-head"));
        assert!(KNOWN_TAGS.contains("mj-section"));
        assert!(KNOWN_TAGS.contains("mj-column"));
        assert!(KNOWN_TAGS.contains("mj-text"));
        assert!(KNOWN_TAGS.contains("mj-image"));
        assert!(KNOWN_TAGS.contains("mj-button"));
        assert!(KNOWN_TAGS.contains("mj-social"));
        assert!(KNOWN_TAGS.contains("mj-social-element"));
        assert!(KNOWN_TAGS.contains("mj-accordion"));
        assert!(KNOWN_TAGS.contains("mj-accordion-element"));
        assert!(KNOWN_TAGS.contains("mj-accordion-title"));
        assert!(KNOWN_TAGS.contains("mj-accordion-text"));
        assert!(KNOWN_TAGS.contains("mj-carousel"));
        assert!(KNOWN_TAGS.contains("mj-carousel-image"));
        assert!(KNOWN_TAGS.contains("mj-navbar"));
        assert!(KNOWN_TAGS.contains("mj-navbar-link"));
        assert!(!KNOWN_TAGS.contains("mj-nonexistent"));
    }

    #[test]
    fn test_allowed_parents_mj_text() {
        let parents = allowed_parents("mj-text");
        assert!(parents.is_some());
        let parents = parents.unwrap();
        assert!(parents.contains(&"mj-column"));
        assert!(parents.contains(&"mj-hero"));
        assert!(!parents.contains(&"mj-body"));
    }

    #[test]
    fn test_allowed_parents_mj_head() {
        let parents = allowed_parents("mj-head");
        assert!(parents.is_some());
        assert!(parents.unwrap().contains(&"mjml"));
    }

    #[test]
    fn test_allowed_parents_unknown_tag() {
        assert!(allowed_parents("div").is_none());
    }

    #[test]
    fn test_required_attrs_mj_image() {
        let attrs = required_attributes("mj-image");
        assert!(attrs.is_some());
        assert!(attrs.unwrap().contains(&"src"));
    }

    #[test]
    fn test_required_attrs_mj_text() {
        // mj-text has no required attributes
        assert!(required_attributes("mj-text").is_none());
    }

    #[test]
    fn test_suggest_tag_typo() {
        assert_eq!(suggest_tag("mj-seciton"), Some("mj-section"));
        assert_eq!(suggest_tag("mj-buton"), Some("mj-button"));
        assert_eq!(suggest_tag("mj-section"), None); // exact match = no suggestion needed
        assert_eq!(suggest_tag("mj-xyzabc"), None); // too far
    }
}
```

**Step 2: Run tests — expect FAIL (module doesn't exist)**

Run: `cargo test --manifest-path crates/mjml-lsp/Cargo.toml`

**Step 3: Implement `rules.rs`**

```rust
use std::collections::HashSet;
use std::sync::LazyLock;

/// All known MJML tag names.
pub static KNOWN_TAGS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "mjml",
        "mj-head", "mj-body",
        "mj-title", "mj-preview", "mj-style", "mj-font", "mj-breakpoint",
        "mj-attributes", "mj-html-attributes",
        "mj-all", "mj-class",
        "mj-selector", "mj-html-attribute",
        "mj-section", "mj-wrapper", "mj-hero", "mj-group", "mj-column",
        "mj-text", "mj-image", "mj-button", "mj-divider", "mj-spacer",
        "mj-social", "mj-social-element",
        "mj-accordion", "mj-accordion-element", "mj-accordion-title", "mj-accordion-text",
        "mj-carousel", "mj-carousel-image",
        "mj-navbar", "mj-navbar-link",
        "mj-table", "mj-raw",
        "mj-include",
    ])
});

/// Returns allowed parent tag names for a given MJML tag, or None if not an MJML tag.
pub fn allowed_parents(tag: &str) -> Option<&'static [&'static str]> {
    match tag {
        "mj-head" | "mj-body" => Some(&["mjml"]),
        "mj-title" | "mj-preview" | "mj-style" | "mj-font"
        | "mj-breakpoint" | "mj-attributes" | "mj-html-attributes" => Some(&["mj-head"]),
        "mj-all" | "mj-class" => Some(&["mj-attributes"]),
        "mj-selector" => Some(&["mj-html-attributes"]),
        "mj-html-attribute" => Some(&["mj-selector"]),
        "mj-section" => Some(&["mj-body", "mj-wrapper"]),
        "mj-wrapper" | "mj-hero" => Some(&["mj-body"]),
        "mj-group" => Some(&["mj-section"]),
        "mj-column" => Some(&["mj-section", "mj-group"]),
        "mj-text" | "mj-image" | "mj-button" | "mj-divider" | "mj-spacer"
        | "mj-social" | "mj-accordion" | "mj-carousel" | "mj-table"
        | "mj-navbar" => Some(&["mj-column", "mj-hero"]),
        "mj-raw" => Some(&["mj-head", "mj-column", "mj-hero"]),
        "mj-social-element" => Some(&["mj-social"]),
        "mj-accordion-element" => Some(&["mj-accordion"]),
        "mj-accordion-title" | "mj-accordion-text" => Some(&["mj-accordion-element"]),
        "mj-carousel-image" => Some(&["mj-carousel"]),
        "mj-navbar-link" => Some(&["mj-navbar"]),
        "mj-include" => Some(&["mj-head", "mj-body", "mj-column", "mj-hero"]),
        _ => None,
    }
}

/// Returns required attribute names for a given MJML tag, or None if none required.
pub fn required_attributes(tag: &str) -> Option<&'static [&'static str]> {
    match tag {
        "mj-image" | "mj-carousel-image" => Some(&["src"]),
        "mj-font" => Some(&["name", "href"]),
        "mj-breakpoint" => Some(&["width"]),
        "mj-class" => Some(&["name"]),
        "mj-social-element" => Some(&["name"]),
        _ => None,
    }
}

/// If `tag` is an unknown mj-* tag close to a known one (edit distance <= 2),
/// returns the closest known tag. Returns None if exact match or no close match.
pub fn suggest_tag(tag: &str) -> Option<&'static str> {
    if KNOWN_TAGS.contains(tag) {
        return None; // exact match, no suggestion needed
    }
    let mut best: Option<(&str, usize)> = None;
    for &known in KNOWN_TAGS.iter() {
        let d = edit_distance(tag, known);
        if d <= 2 {
            if best.is_none() || d < best.unwrap().1 {
                best = Some((known, d));
            }
        }
    }
    best.map(|(tag, _)| tag)
}

/// Simple Levenshtein distance.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in 0..=a.len() { dp[i][0] = i; }
    for j in 0..=b.len() { dp[0][j] = j; }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}
```

**Step 4: Add `mod rules;` to `main.rs`**

Add after the existing `use` statements at top of `main.rs`:
```rust
mod rules;
```

**Step 5: Run tests — expect PASS**

Run: `cargo test --manifest-path crates/mjml-lsp/Cargo.toml`

**Step 6: Commit**

```bash
git add crates/mjml-lsp/src/rules.rs crates/mjml-lsp/src/main.rs
git commit -m "feat(lsp): add MJML spec rules — known tags, nesting, required attributes"
```

---

### Task 2: Create `scanner.rs` — lightweight tag scanner

**Files:**
- Create: `crates/mjml-lsp/src/scanner.rs`
- Modify: `crates/mjml-lsp/src/main.rs` (add `mod scanner;`)

**Step 1: Write the tests**

Add to bottom of `scanner.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_simple_tag() {
        let tags = scan_tags("<mjml></mjml>");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "mjml");
        assert!(!tags[0].self_closing);
        assert!(tags[0].parent_idx.is_none());
    }

    #[test]
    fn test_scan_self_closing() {
        let tags = scan_tags("<mj-image src=\"x\" />");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "mj-image");
        assert!(tags[0].self_closing);
        assert_eq!(tags[0].attributes.len(), 1);
        assert_eq!(tags[0].attributes[0].name, "src");
        assert_eq!(tags[0].attributes[0].value.as_deref(), Some("x"));
    }

    #[test]
    fn test_scan_nested() {
        let tags = scan_tags("<mjml><mj-body><mj-section /></mj-body></mjml>");
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].name, "mjml");
        assert_eq!(tags[1].name, "mj-body");
        assert_eq!(tags[1].parent_idx, Some(0));
        assert_eq!(tags[2].name, "mj-section");
        assert_eq!(tags[2].parent_idx, Some(1));
    }

    #[test]
    fn test_scan_skips_comments() {
        let tags = scan_tags("<mjml><!-- comment --><mj-body /></mjml>");
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "mjml");
        assert_eq!(tags[1].name, "mj-body");
    }

    #[test]
    fn test_scan_skips_mj_style_content() {
        let tags = scan_tags("<mjml><mj-head><mj-style>.foo { color: red; }</mj-style></mj-head></mjml>");
        // Should have mjml, mj-head, mj-style — but NOT parse CSS as tags
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[2].name, "mj-style");
    }

    #[test]
    fn test_scan_attributes_with_positions() {
        let text = "<mj-image src=\"hello\" alt=\"world\" />";
        let tags = scan_tags(text);
        assert_eq!(tags[0].attributes.len(), 2);
        let src_attr = &tags[0].attributes[0];
        assert_eq!(src_attr.name, "src");
        // The attribute name "src" starts at some position in the source
        let extracted = &text[src_attr.name_span.0..src_attr.name_span.1];
        assert_eq!(extracted, "src");
    }

    #[test]
    fn test_scan_tag_span() {
        let text = "<mj-image src=\"x\" />";
        let tags = scan_tags(text);
        assert_eq!(tags[0].tag_span, (0, text.len()));
    }

    #[test]
    fn test_scan_empty() {
        let tags = scan_tags("");
        assert!(tags.is_empty());
    }

    #[test]
    fn test_scan_plain_text_ignored() {
        let tags = scan_tags("just some text");
        assert!(tags.is_empty());
    }

    #[test]
    fn test_scan_non_mj_tags_inside_content() {
        // Non-mj tags inside mj-text are valid HTML content — scanner still records them
        let tags = scan_tags("<mjml><mj-body><mj-section><mj-column><mj-text><p>Hi</p></mj-text></mj-column></mj-section></mj-body></mjml>");
        let tag_names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
        assert!(tag_names.contains(&"p")); // scanner sees it, validator ignores non-mj-* tags
    }

    #[test]
    fn test_scan_attribute_without_value() {
        let tags = scan_tags("<mj-style inline></mj-style>");
        assert_eq!(tags[0].attributes.len(), 1);
        assert_eq!(tags[0].attributes[0].name, "inline");
        assert!(tags[0].attributes[0].value.is_none());
    }
}
```

**Step 2: Run tests — expect FAIL**

Run: `cargo test --manifest-path crates/mjml-lsp/Cargo.toml`

**Step 3: Implement `scanner.rs`**

```rust
/// Represents a parsed opening tag with its source position.
#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    /// Byte range of the full opening tag (from `<` to `>`).
    pub tag_span: (usize, usize),
    pub self_closing: bool,
    pub attributes: Vec<AttrInfo>,
    /// Index into the `Vec<TagInfo>` of the parent opening tag, if any.
    pub parent_idx: Option<usize>,
}

/// Represents a parsed attribute with its source position.
#[derive(Debug, Clone)]
pub struct AttrInfo {
    pub name: String,
    pub value: Option<String>,
    /// Byte range of the attribute name in the source.
    pub name_span: (usize, usize),
}

/// Scans source text and extracts all opening tags with their attributes and
/// parent-child relationships. Skips HTML comments and content inside `<mj-style>`.
pub fn scan_tags(text: &str) -> Vec<TagInfo> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut tags: Vec<TagInfo> = Vec::new();
    let mut parent_stack: Vec<usize> = Vec::new(); // indices into `tags`
    let mut pos = 0;

    while pos < len {
        if bytes[pos] != b'<' {
            pos += 1;
            continue;
        }

        // Skip comments: <!-- ... -->
        if pos + 3 < len && &bytes[pos..pos + 4] == b"<!--" {
            if let Some(end) = find_bytes(bytes, pos + 4, b"-->") {
                pos = end + 3;
            } else {
                break; // unclosed comment, stop
            }
            continue;
        }

        // Closing tag: </name>
        if pos + 1 < len && bytes[pos + 1] == b'/' {
            if let Some(gt) = memchr(b'>', bytes, pos + 2) {
                let tag_name = extract_tag_name(bytes, pos + 2);
                // Pop the parent stack if it matches
                if !tag_name.is_empty() {
                    if let Some(&top_idx) = parent_stack.last() {
                        if tags[top_idx].name == tag_name {
                            parent_stack.pop();
                        }
                    }
                }
                pos = gt + 1;
            } else {
                break;
            }
            continue;
        }

        // Opening tag: <name ...> or <name ... />
        let tag_name = extract_tag_name(bytes, pos + 1);
        if tag_name.is_empty() {
            pos += 1;
            continue;
        }

        // Find the closing '>'
        let gt = match find_gt_skipping_strings(bytes, pos + 1 + tag_name.len()) {
            Some(gt) => gt,
            None => break,
        };

        let self_closing = gt > 0 && bytes[gt - 1] == b'/';
        let attr_end = if self_closing { gt - 1 } else { gt };
        let attr_start = pos + 1 + tag_name.len();
        let attributes = parse_attributes(bytes, attr_start, attr_end);

        let parent_idx = parent_stack.last().copied();
        let tag_idx = tags.len();

        tags.push(TagInfo {
            name: tag_name.clone(),
            tag_span: (pos, gt + 1),
            self_closing,
            attributes,
            parent_idx,
        });

        if !self_closing {
            parent_stack.push(tag_idx);
        }

        pos = gt + 1;

        // Skip content inside <mj-style>...</mj-style>
        if tag_name == "mj-style" && !self_closing {
            if let Some(end) = find_closing_tag(bytes, pos, "mj-style") {
                // Pop the parent stack for mj-style
                if let Some(&top_idx) = parent_stack.last() {
                    if tags[top_idx].name == "mj-style" {
                        parent_stack.pop();
                    }
                }
                pos = end;
            }
        }
    }

    tags
}

fn extract_tag_name(bytes: &[u8], start: usize) -> String {
    let mut end = start;
    while end < bytes.len() {
        let b = bytes[end];
        if b.is_ascii_alphanumeric() || b == b'-' {
            end += 1;
        } else {
            break;
        }
    }
    String::from_utf8_lossy(&bytes[start..end]).to_string()
}

fn parse_attributes(bytes: &[u8], start: usize, end: usize) -> Vec<AttrInfo> {
    let mut attrs = Vec::new();
    let mut pos = start;

    while pos < end {
        // Skip whitespace
        while pos < end && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= end {
            break;
        }

        // Read attribute name
        let name_start = pos;
        while pos < end && bytes[pos] != b'=' && !bytes[pos].is_ascii_whitespace() && bytes[pos] != b'/' {
            pos += 1;
        }
        if pos == name_start {
            pos += 1;
            continue;
        }
        let name = String::from_utf8_lossy(&bytes[name_start..pos]).to_string();
        let name_span = (name_start, pos);

        // Skip whitespace
        while pos < end && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }

        // Check for '='
        if pos < end && bytes[pos] == b'=' {
            pos += 1;
            // Skip whitespace
            while pos < end && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            // Read value
            if pos < end && (bytes[pos] == b'"' || bytes[pos] == b'\'') {
                let quote = bytes[pos];
                pos += 1;
                let val_start = pos;
                while pos < end && bytes[pos] != quote {
                    pos += 1;
                }
                let value = String::from_utf8_lossy(&bytes[val_start..pos]).to_string();
                if pos < end {
                    pos += 1; // skip closing quote
                }
                attrs.push(AttrInfo { name, value: Some(value), name_span });
            } else {
                // Unquoted value
                let val_start = pos;
                while pos < end && !bytes[pos].is_ascii_whitespace() {
                    pos += 1;
                }
                let value = String::from_utf8_lossy(&bytes[val_start..pos]).to_string();
                attrs.push(AttrInfo { name, value: Some(value), name_span });
            }
        } else {
            // Boolean attribute (no value)
            attrs.push(AttrInfo { name, value: None, name_span });
        }
    }

    attrs
}

fn memchr(needle: u8, haystack: &[u8], start: usize) -> Option<usize> {
    haystack[start..].iter().position(|&b| b == needle).map(|i| start + i)
}

fn find_bytes(haystack: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    haystack[start..].windows(needle.len())
        .position(|w| w == needle)
        .map(|i| start + i)
}

fn find_gt_skipping_strings(bytes: &[u8], start: usize) -> Option<usize> {
    let mut pos = start;
    while pos < bytes.len() {
        match bytes[pos] {
            b'"' | b'\'' => {
                let quote = bytes[pos];
                pos += 1;
                while pos < bytes.len() && bytes[pos] != quote {
                    pos += 1;
                }
                if pos < bytes.len() { pos += 1; }
            }
            b'>' => return Some(pos),
            _ => pos += 1,
        }
    }
    None
}

fn find_closing_tag(bytes: &[u8], start: usize, tag_name: &str) -> Option<usize> {
    let pattern = format!("</{tag_name}>");
    let pat_bytes = pattern.as_bytes();
    find_bytes(bytes, start, pat_bytes).map(|i| i + pat_bytes.len())
}
```

**Step 4: Add `mod scanner;` to `main.rs`**

Add after `mod rules;`:
```rust
mod scanner;
```

**Step 5: Run tests — expect PASS**

Run: `cargo test --manifest-path crates/mjml-lsp/Cargo.toml`

**Step 6: Commit**

```bash
git add crates/mjml-lsp/src/scanner.rs crates/mjml-lsp/src/main.rs
git commit -m "feat(lsp): add tag scanner with byte positions and parent tracking"
```

---

### Task 3: Create `validate.rs` — MJML validation rules

**Files:**
- Create: `crates/mjml-lsp/src/validate.rs`
- Modify: `crates/mjml-lsp/src/main.rs` (add `mod validate;`)

**Step 1: Write the tests**

Add to bottom of `validate.rs`:

```rust
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
        assert!(diags.is_empty(), "valid document should produce no diagnostics, got: {:?}", diags);
    }

    #[test]
    fn test_nesting_mj_text_in_section() {
        let diags = validate("<mjml><mj-body><mj-section><mj-text>Bad</mj-text></mj-section></mj-body></mjml>");
        assert!(!diags.is_empty());
        let d = &diags[0];
        assert_eq!(d.severity, Severity::Error);
        assert!(d.message.contains("<mj-text>"), "msg: {}", d.message);
        assert!(d.message.contains("<mj-column>") || d.message.contains("<mj-hero>"), "msg: {}", d.message);
        assert!(d.message.contains("<mj-section>"), "msg: {}", d.message);
    }

    #[test]
    fn test_nesting_mj_section_in_body_ok() {
        let diags = validate("<mjml><mj-body><mj-section><mj-column /></mj-section></mj-body></mjml>");
        // mj-section in mj-body is valid
        let nesting_errors: Vec<_> = diags.iter().filter(|d| d.message.contains("must be inside")).collect();
        assert!(nesting_errors.is_empty());
    }

    #[test]
    fn test_nesting_mj_column_outside_section() {
        let diags = validate("<mjml><mj-body><mj-column><mj-text>Bad</mj-text></mj-column></mj-body></mjml>");
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
        let unknown_diags: Vec<_> = diags.iter().filter(|d| d.message.contains("Unknown")).collect();
        assert!(!unknown_diags.is_empty());
        assert_eq!(unknown_diags[0].severity, Severity::Warning);
        assert!(unknown_diags[0].message.contains("mj-section"), "should suggest correction, msg: {}", unknown_diags[0].message);
    }

    #[test]
    fn test_unknown_non_mj_tag_ignored() {
        let diags = validate("<mjml><mj-body><mj-section><mj-column><mj-text><div>ok</div></mj-text></mj-column></mj-section></mj-body></mjml>");
        let unknown_diags: Vec<_> = diags.iter().filter(|d| d.message.contains("Unknown")).collect();
        assert!(unknown_diags.is_empty(), "non-mj tags should not trigger unknown tag warning");
    }

    #[test]
    fn test_duplicate_mj_body() {
        let diags = validate("<mjml><mj-body></mj-body><mj-body></mj-body></mjml>");
        let dup_diags: Vec<_> = diags.iter().filter(|d| d.message.contains("Duplicate")).collect();
        assert!(!dup_diags.is_empty());
        assert_eq!(dup_diags[0].severity, Severity::Error);
    }

    #[test]
    fn test_duplicate_mj_head() {
        let diags = validate("<mjml><mj-head></mj-head><mj-head></mj-head></mjml>");
        let dup_diags: Vec<_> = diags.iter().filter(|d| d.message.contains("Duplicate")).collect();
        assert!(!dup_diags.is_empty());
    }

    #[test]
    fn test_reports_multiple_errors() {
        // Two nesting violations in one document
        let diags = validate("<mjml><mj-body><mj-section><mj-text>A</mj-text><mj-image src=\"x\" /></mj-section></mj-body></mjml>");
        let nesting: Vec<_> = diags.iter().filter(|d| d.message.contains("must be inside")).collect();
        assert!(nesting.len() >= 2, "should report both nesting errors, got {}", nesting.len());
    }

    #[test]
    fn test_required_attr_mj_font() {
        let diags = validate("<mjml><mj-head><mj-font /></mj-head></mjml>");
        let attr_diags: Vec<_> = diags.iter().filter(|d| d.message.contains("missing required")).collect();
        assert!(attr_diags.len() >= 2, "mj-font requires name and href, got {} warnings", attr_diags.len());
    }
}
```

**Step 2: Run tests — expect FAIL**

Run: `cargo test --manifest-path crates/mjml-lsp/Cargo.toml`

**Step 3: Implement `validate.rs`**

```rust
use crate::rules::{self, KNOWN_TAGS};
use crate::scanner::TagInfo;

/// Severity levels for lint diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A lint diagnostic produced by the tag validator.
#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    /// Byte range in the source text to underline.
    pub span: (usize, usize),
    pub severity: Severity,
    pub message: String,
}

/// Validates a list of scanned tags against MJML rules.
/// Returns all violations found (does not stop at the first).
pub fn validate_tags(text: &str, tags: &[TagInfo]) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();

    let mut head_count = 0u32;
    let mut body_count = 0u32;

    for (idx, tag) in tags.iter().enumerate() {
        // Rule 4: Singleton enforcement
        if tag.name == "mj-head" {
            head_count += 1;
            if head_count > 1 {
                diagnostics.push(LintDiagnostic {
                    span: tag.tag_span,
                    severity: Severity::Error,
                    message: "Duplicate <mj-head> — only one <mj-head> is allowed per document".to_string(),
                });
            }
        }
        if tag.name == "mj-body" {
            body_count += 1;
            if body_count > 1 {
                diagnostics.push(LintDiagnostic {
                    span: tag.tag_span,
                    severity: Severity::Error,
                    message: "Duplicate <mj-body> — only one <mj-body> is allowed per document".to_string(),
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
            if let Some(suggestion) = rules::suggest_tag(&tag.name) {
                msg.push_str(&format!(" — did you mean <{suggestion}>?"));
            }
            diagnostics.push(LintDiagnostic {
                span: tag.tag_span,
                severity: Severity::Warning,
                message: msg,
            });
            continue; // skip nesting/attr checks for unknown tags
        }

        // Rule 1: Nesting
        if let Some(allowed) = rules::allowed_parents(&tag.name) {
            let actual_parent = tag.parent_idx.map(|i| tags[i].name.as_str());
            let is_valid = match actual_parent {
                Some(parent_name) => allowed.contains(&parent_name),
                None => false, // no parent means it's at root level (only "mjml" is valid there)
            };
            // "mjml" has no parent entry in allowed_parents, so skip it
            if !is_valid && tag.name != "mjml" {
                let parent_display = actual_parent.unwrap_or("document root");
                let expected = allowed.iter()
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
                });
            }
        }

        // Rule 2: Required attributes
        if let Some(required) = rules::required_attributes(&tag.name) {
            let present: Vec<&str> = tag.attributes.iter().map(|a| a.name.as_str()).collect();
            for &attr_name in required {
                if !present.contains(&attr_name) {
                    diagnostics.push(LintDiagnostic {
                        span: tag.tag_span,
                        severity: Severity::Warning,
                        message: format!(
                            "<{}> is missing required attribute \"{}\"",
                            tag.name, attr_name
                        ),
                    });
                }
            }
        }
    }

    diagnostics
}
```

**Step 4: Add `mod validate;` to `main.rs`**

Add after `mod scanner;`:
```rust
mod validate;
```

**Step 5: Run tests — expect PASS**

Run: `cargo test --manifest-path crates/mjml-lsp/Cargo.toml`

**Step 6: Commit**

```bash
git add crates/mjml-lsp/src/validate.rs crates/mjml-lsp/src/main.rs
git commit -m "feat(lsp): add MJML validation rules — nesting, required attrs, unknown tags, singletons"
```

---

### Task 4: Wire validation into `main.rs`

**Files:**
- Modify: `crates/mjml-lsp/src/main.rs`

**Step 1: Write the integration test**

Add to the existing `tests` module in `main.rs`:

```rust
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

    // Existing test should now produce MORE diagnostics (scanner + mrml)
    #[test]
    fn test_validate_mjml_valid_document_still_clean() {
        let text = "<mjml><mj-head><mj-title>Hi</mj-title></mj-head><mj-body><mj-section><mj-column><mj-text>Hello</mj-text></mj-column></mj-section></mj-body></mjml>";
        let diagnostics = validate_mjml(text);
        assert!(diagnostics.is_empty(), "valid document should have no diagnostics, got: {:?}", diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>());
    }
```

**Step 2: Run tests — expect FAIL (validate_mjml doesn't call scanner yet)**

Run: `cargo test --manifest-path crates/mjml-lsp/Cargo.toml`

**Step 3: Modify `validate_mjml()` in `main.rs`**

Replace the existing `validate_mjml` function:

```rust
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
```

**Step 4: Run ALL tests — expect PASS**

Run: `cargo test --manifest-path crates/mjml-lsp/Cargo.toml`

Some existing tests may need updating:
- `test_validate_mjml_unknown_element_is_accepted` — now our scanner will flag unknown `mj-*` tags. Update to use a non-`mj-*` tag like `<div>` instead of `<invalid-tag>`.

**Step 5: Rebuild debug binary**

Run: `cargo build --manifest-path crates/mjml-lsp/Cargo.toml`

**Step 6: Commit**

```bash
git add crates/mjml-lsp/src/main.rs
git commit -m "feat(lsp): wire tag scanner validation into LSP diagnostic pipeline"
```

---

### Task 5: Update test files

**Files:**
- Modify: `test/invalid.mjml`

**Step 1: Rewrite invalid.mjml to exercise all rules**

```html
<!-- Tests all 4 validation rules. Multiple errors should be reported. -->
<mjml>
  <mj-head>
    <mj-title>Broken Email</mj-title>
    <!-- Rule 2: mj-font missing required name and href -->
    <mj-font />
  </mj-head>
  <mj-body>
    <!-- Rule 1: mj-text must be inside mj-column, not mj-section -->
    <mj-section>
      <mj-text>Wrong parent</mj-text>
    </mj-section>

    <!-- Rule 1: mj-column must be inside mj-section, not mj-body -->
    <mj-column>
      <mj-text>Orphan column</mj-text>
    </mj-column>

    <!-- Rule 2: mj-image missing required src -->
    <mj-section>
      <mj-column>
        <mj-image alt="No source" />
      </mj-column>
    </mj-section>

    <!-- Rule 3: unknown mj-* tag (typo) -->
    <mj-section>
      <mj-column>
        <mj-buton href="https://example.com">Click</mj-buton>
      </mj-column>
    </mj-section>
  </mj-body>

  <!-- Rule 4: duplicate mj-body -->
  <mj-body>
    <mj-section>
      <mj-column>
        <mj-text>Second body</mj-text>
      </mj-column>
    </mj-section>
  </mj-body>
</mjml>
```

**Step 2: Verify valid.mjml is still clean**

Run: `cargo build --manifest-path crates/mjml-lsp/Cargo.toml`
Then test in Zed: open both files.

**Step 3: Commit**

```bash
git add test/
git commit -m "test: update test files to exercise all validation rules"
```

---

### Task 6: Clean up old test files

**Files:**
- Delete: `test/invalid-no-root.mjml`
- Delete: `test/invalid-unclosed-tag.mjml`
- Delete: `test/invalid-bad-xml.mjml`
- Delete: `test/invalid-text-in-image.mjml`
- Delete: `test/invalid-empty.mjml`

These were workarounds for mrml's single-error limitation. Now that the scanner reports all errors, a single `invalid.mjml` suffices.

**Step 1: Delete files and commit**

```bash
rm test/invalid-no-root.mjml test/invalid-unclosed-tag.mjml test/invalid-bad-xml.mjml test/invalid-text-in-image.mjml test/invalid-empty.mjml
git add -A test/
git commit -m "chore: remove single-error test files, consolidated into invalid.mjml"
```
