use std::collections::HashMap;

use lsp_server::{Request, Response};
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind, Range, Uri};

use crate::rules;
use crate::scanner;

/// Handles a `textDocument/hover` request and returns the LSP response.
pub fn handle(req: &Request, documents: &HashMap<Uri, String>) -> Response {
    Response::new_ok(req.id.clone(), hover_result(req, documents))
}

fn hover_result(req: &Request, documents: &HashMap<Uri, String>) -> Option<Hover> {
    let params = serde_json::from_value::<HoverParams>(req.params.clone()).ok()?;
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;
    let text = documents.get(&uri)?;
    let offset = crate::position_to_offset(text, position);
    hover(text, offset)
}

/// Produces hover documentation for the token at `offset` (a byte offset).
fn hover(text: &str, offset: usize) -> Option<Hover> {
    let tags = scanner::scan_tags(text);
    let tag = tags
        .iter()
        .find(|t| t.tag_span.0 <= offset && offset < t.tag_span.1)?;

    // Is the cursor on the tag name?
    let name_start = tag.tag_span.0 + 1;
    let name_end = name_start + tag.name.len();
    if (name_start..=name_end).contains(&offset) {
        return tag_hover(&tag.name, text, name_start, name_end);
    }

    // Is the cursor on one of the attribute names?
    for attr in &tag.attributes {
        if (attr.name_span.0..=attr.name_span.1).contains(&offset) {
            return attr_hover(&tag.name, &attr.name, text, attr.name_span);
        }
    }

    None
}

fn tag_hover(name: &str, text: &str, start: usize, end: usize) -> Option<Hover> {
    let spec = rules::component(name)?;
    let mut value = format!("### {}\n\n{}", spec.name, spec.doc);
    if !spec.attributes.is_empty() {
        let attrs: Vec<&str> = spec.attributes.iter().map(|a| a.name).collect();
        value.push_str("\n\n**Attributes:** ");
        value.push_str(&attrs.join(", "));
    }
    value.push_str("\n\n[MJML documentation](");
    value.push_str(spec.docs_url);
    value.push(')');
    Some(markup_hover(value, text, start, end))
}

fn attr_hover(tag: &str, attr: &str, text: &str, span: (usize, usize)) -> Option<Hover> {
    let spec = rules::attribute(tag, attr)?;
    let mut value = format!(
        "### {}\n\n`{}` on `<{tag}>`\n\n{}",
        spec.name,
        spec.ty.label(),
        spec.doc
    );
    if let Some(default) = spec.default {
        value.push_str("\n\nDefault: `");
        value.push_str(default);
        value.push('`');
    }
    Some(markup_hover(value, text, span.0, span.1))
}

fn markup_hover(value: String, text: &str, start: usize, end: usize) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: Some(Range::new(
            crate::byte_offset_to_position(text, start),
            crate::byte_offset_to_position(text, end),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the hover at the `|` marker in `src` (ASCII only).
    fn hover_at(src: &str) -> Option<Hover> {
        let offset = src.find('|').expect("test input must contain a `|` marker");
        let text = src.replacen('|', "", 1);
        hover(&text, offset)
    }

    fn markdown(h: &Hover) -> &str {
        match &h.contents {
            HoverContents::Markup(m) => &m.value,
            _ => panic!("expected markup hover"),
        }
    }

    #[test]
    fn tag_hover_shows_component_documentation() {
        let h = hover_at("<mjml><mj-bo|dy></mj-body></mjml>").expect("hover on tag name");
        let md = markdown(&h);
        assert!(md.contains("mj-body"));
        assert!(md.contains("documentation.mjml.io"));
    }

    #[test]
    fn attribute_hover_shows_type_and_owner() {
        let h = hover_at("<mj-image sr|c=\"x.png\" />").expect("hover on attribute name");
        let md = markdown(&h);
        assert!(md.contains("src"));
        assert!(md.contains("url"));
        assert!(md.contains("<mj-image>"));
    }

    #[test]
    fn attribute_hover_includes_default_when_present() {
        let h = hover_at("<mj-text al|ign=\"center\">Hi</mj-text>").expect("hover on align");
        let md = markdown(&h);
        assert!(md.contains("left | right | center | justify"));
        assert!(md.contains("Default:"));
    }

    #[test]
    fn no_hover_in_text_content() {
        assert!(hover_at("<mjml>te|xt</mjml>").is_none());
    }

    #[test]
    fn no_hover_for_unknown_tag_attribute() {
        assert!(hover_at("<div cla|ss=\"x\"></div>").is_none());
    }

    #[test]
    fn hover_range_covers_the_tag_name() {
        let h = hover_at("<mjml><mj-bo|dy></mj-body></mjml>").unwrap();
        let range = h.range.expect("hover should carry a range");
        // "mj-body" starts at byte 7 on line 0.
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 7);
        assert_eq!(range.end.character, 14);
    }
}
