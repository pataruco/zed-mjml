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
