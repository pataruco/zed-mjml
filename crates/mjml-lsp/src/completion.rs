use std::collections::{HashMap, HashSet};

use lsp_server::{Request, RequestId, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, CompletionTextEdit,
    Documentation, InsertTextFormat, MarkupContent, MarkupKind, Range, TextEdit, Uri,
};

use crate::rules::{self, AttrSpec, AttrType, ComponentSpec};

/// Handles a `textDocument/completion` request and returns the LSP response.
pub fn handle(req: &Request, documents: &HashMap<Uri, String>) -> Response {
    let Ok(params) = serde_json::from_value::<CompletionParams>(req.params.clone()) else {
        return ok_empty(&req.id);
    };
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;
    let Some(text) = documents.get(&uri) else {
        return ok_empty(&req.id);
    };
    let offset = crate::position_to_offset(text, position);
    let items = complete(text, offset);
    Response::new_ok(req.id.clone(), CompletionResponse::Array(items))
}

fn ok_empty(id: &RequestId) -> Response {
    Response::new_ok(id.clone(), CompletionResponse::Array(Vec::new()))
}

/// What the cursor is positioned to complete.
enum Ctx {
    /// Typing a tag name, e.g. `<mj-sec|`.
    Tag {
        parent: Option<String>,
        word: (usize, usize),
    },
    /// Typing an attribute name inside an open tag, e.g. `<mj-section pad|`.
    Attr {
        tag: String,
        present: Vec<String>,
        word: (usize, usize),
    },
    /// Typing an attribute value, e.g. `<mj-text align="|"`.
    Value {
        tag: String,
        attr: String,
        word: (usize, usize),
    },
    /// Nothing useful to complete here.
    None,
}

/// Produces completion items for the cursor at `offset` (a byte offset).
fn complete(text: &str, offset: usize) -> Vec<CompletionItem> {
    match detect_context(text, offset) {
        Ctx::Tag { parent, word } => tag_items(parent.as_deref(), word_range(text, word)),
        Ctx::Attr { tag, present, word } => attr_items(&tag, &present, word_range(text, word)),
        Ctx::Value { tag, attr, word } => value_items(&tag, &attr, word_range(text, word)),
        Ctx::None => Vec::new(),
    }
}

/// Classifies the completion context by scanning backwards from the cursor.
fn detect_context(text: &str, offset: usize) -> Ctx {
    let bytes = text.as_bytes();
    let off = offset.min(bytes.len());

    let Some(lt) = bytes[..off].iter().rposition(|&b| b == b'<') else {
        return Ctx::None;
    };
    if let Some(gt) = bytes[..off].iter().rposition(|&b| b == b'>') {
        if gt > lt {
            // The most recent tag was already closed; the cursor is in text content.
            return Ctx::None;
        }
    }

    let after = lt + 1;
    // Just typed `<` (possibly at end of input): offer tags.
    if after >= off {
        return Ctx::Tag {
            parent: crate::find_parent_tag(text, lt),
            word: (after, off),
        };
    }
    // Closing tag (`</`), comment, or declaration (`<!`): nothing to complete.
    if bytes[after] == b'/' || bytes[after] == b'!' {
        return Ctx::None;
    }

    // Consume the tag name.
    let mut i = after;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
        i += 1;
    }
    let name_end = i;
    if off <= name_end {
        // Cursor is within the tag name.
        return Ctx::Tag {
            parent: crate::find_parent_tag(text, lt),
            word: (after, off),
        };
    }

    let tag = String::from_utf8_lossy(&bytes[after..name_end]).into_owned();
    tag_region_context(bytes, name_end, off, tag)
}

/// Classifies the cursor within the attribute region of an open tag.
fn tag_region_context(bytes: &[u8], name_end: usize, offset: usize, tag: String) -> Ctx {
    let mut present: Vec<String> = Vec::new();
    let mut i = name_end;
    loop {
        while i < offset && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= offset {
            // In whitespace between attributes: offer attribute names.
            return Ctx::Attr {
                tag,
                present,
                word: (offset, offset),
            };
        }

        let name_start = i;
        while i < offset && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
            i += 1;
        }
        if i >= offset {
            // Cursor is in the middle of typing an attribute name.
            return Ctx::Attr {
                tag,
                present,
                word: (name_start, offset),
            };
        }
        if i == name_start {
            // Not an attribute-name character (e.g. `/` or `>`); skip it.
            i += 1;
            continue;
        }

        let attr_name = String::from_utf8_lossy(&bytes[name_start..i]).into_owned();
        while i < offset && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        // Consume an optional value so the next iteration starts on a fresh attribute.
        if i < offset && bytes[i] == b'=' {
            i += 1;
            while i < offset && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < offset && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                i += 1;
                let val_start = i;
                while i < offset && bytes[i] != quote {
                    i += 1;
                }
                if i >= offset {
                    // Cursor is inside this attribute's value.
                    return Ctx::Value {
                        tag,
                        attr: attr_name,
                        word: (val_start, offset),
                    };
                }
                i += 1; // consume closing quote
            } else {
                // Unquoted value: consume up to the next whitespace.
                while i < offset && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
            }
        }

        present.push(attr_name);
    }
}

fn word_range(text: &str, span: (usize, usize)) -> Range {
    Range::new(
        crate::byte_offset_to_position(text, span.0),
        crate::byte_offset_to_position(text, span.1),
    )
}

/// Tag-name completions. Valid children of `parent` are sorted first.
fn tag_items(parent: Option<&str>, range: Range) -> Vec<CompletionItem> {
    let valid: HashSet<&'static str> = parent.map_or_else(HashSet::new, |p| {
        rules::allowed_children(p).into_iter().collect()
    });
    rules::REGISTRY
        .values()
        .copied()
        .map(|spec| {
            let rank = if valid.contains(spec.name) { '0' } else { '1' };
            CompletionItem {
                label: spec.name.to_string(),
                kind: Some(CompletionItemKind::CLASS),
                detail: Some(spec.doc.to_string()),
                documentation: Some(component_doc(spec)),
                sort_text: Some(format!("{rank}{}", spec.name)),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range,
                    new_text: spec.name.to_string(),
                })),
                ..Default::default()
            }
        })
        .collect()
}

/// Attribute-name completions for `tag`, excluding attributes already present.
fn attr_items(tag: &str, present: &[String], range: Range) -> Vec<CompletionItem> {
    let Some(spec) = rules::component(tag) else {
        return Vec::new();
    };
    spec.attributes
        .iter()
        .filter(|a| !present.iter().any(|p| p.as_str() == a.name))
        .map(|a| CompletionItem {
            label: a.name.to_string(),
            kind: Some(CompletionItemKind::FIELD),
            detail: Some(type_label(a.ty)),
            documentation: Some(attr_doc(a)),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: format!("{}=\"$0\"", a.name),
            })),
            ..Default::default()
        })
        .collect()
}

/// Attribute-value completions: only enumerated attributes offer suggestions.
fn value_items(tag: &str, attr: &str, range: Range) -> Vec<CompletionItem> {
    let Some(a) = rules::attribute(tag, attr) else {
        return Vec::new();
    };
    let AttrType::Enum(values) = a.ty else {
        return Vec::new();
    };
    values
        .iter()
        .map(|value| CompletionItem {
            label: (*value).to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: (*value).to_string(),
            })),
            ..Default::default()
        })
        .collect()
}

fn component_doc(spec: &ComponentSpec) -> Documentation {
    Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value: format!("{}\n\n[MJML documentation]({})", spec.doc, spec.docs_url),
    })
}

fn attr_doc(a: &AttrSpec) -> Documentation {
    let mut value = a.doc.to_string();
    if let Some(default) = a.default {
        value.push_str("\n\nDefault: `");
        value.push_str(default);
        value.push('`');
    }
    Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value,
    })
}

fn type_label(ty: AttrType) -> String {
    match ty {
        AttrType::Text => "text".to_string(),
        AttrType::Color => "color".to_string(),
        AttrType::Measure => "measure".to_string(),
        AttrType::Url => "url".to_string(),
        AttrType::Boolean => "boolean".to_string(),
        AttrType::Enum(values) => values.join(" | "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Completes at the `|` marker in `src` (ASCII only).
    fn complete_at(src: &str) -> Vec<CompletionItem> {
        let offset = src.find('|').expect("test input must contain a `|` marker");
        let text = src.replacen('|', "", 1);
        complete(&text, offset)
    }

    fn labels(items: &[CompletionItem]) -> Vec<String> {
        items.iter().map(|i| i.label.clone()).collect()
    }

    #[test]
    fn tag_completion_after_open_bracket() {
        let l = labels(&complete_at("<mjml><mj-body><|"));
        assert!(l.contains(&"mj-section".to_string()));
        assert!(l.contains(&"mj-wrapper".to_string()));
        assert!(l.contains(&"mj-hero".to_string()));
    }

    #[test]
    fn tag_completion_ranks_valid_children_first() {
        let items = complete_at("<mjml><mj-body><|");
        let section = items.iter().find(|i| i.label == "mj-section").unwrap();
        let text = items.iter().find(|i| i.label == "mj-text").unwrap();
        // mj-section is a valid child of mj-body; mj-text is not, so it sorts later.
        assert!(section.sort_text.as_deref().unwrap() < text.sort_text.as_deref().unwrap());
    }

    #[test]
    fn tag_completion_partial_name() {
        let l = labels(&complete_at("<mjml><mj-body><mj-se|"));
        assert!(l.contains(&"mj-section".to_string()));
    }

    #[test]
    fn attribute_completion_offers_tag_attributes() {
        let l = labels(&complete_at("<mj-section |"));
        assert!(l.contains(&"background-color".to_string()));
        assert!(l.contains(&"padding".to_string()));
        assert!(l.contains(&"text-align".to_string()));
    }

    #[test]
    fn attribute_completion_excludes_present_attributes() {
        let l = labels(&complete_at("<mj-section background-color=\"red\" |"));
        assert!(!l.contains(&"background-color".to_string()));
        assert!(l.contains(&"padding".to_string()));
    }

    #[test]
    fn attribute_completion_partial_name() {
        let l = labels(&complete_at("<mj-image sr|"));
        assert!(l.contains(&"src".to_string()));
    }

    #[test]
    fn value_completion_for_enum_attribute() {
        let l = labels(&complete_at("<mj-text align=\"|\""));
        assert_eq!(l, vec!["left", "right", "center", "justify"]);
    }

    #[test]
    fn value_completion_empty_for_non_enum_attribute() {
        assert!(complete_at("<mj-image src=\"|\"").is_empty());
    }

    #[test]
    fn no_completion_in_text_content() {
        assert!(complete_at("<mjml>hello |").is_empty());
    }

    #[test]
    fn no_completion_in_closing_tag() {
        assert!(complete_at("<mjml></mj|").is_empty());
    }

    #[test]
    fn position_offset_round_trips() {
        let text = "<mjml>\n  <mj-body>\n";
        for off in [0usize, 6, 9, text.len()] {
            let pos = crate::byte_offset_to_position(text, off);
            assert_eq!(crate::position_to_offset(text, pos), off);
        }
    }
}
