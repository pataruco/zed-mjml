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
