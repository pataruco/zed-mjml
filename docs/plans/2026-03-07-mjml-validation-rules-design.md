# MJML Full Validation Rules — Design Document

**Date:** 2026-03-07
**Approach:** B — Pre-parse tag scanner + MJML semantic validation

## Problem

The current LSP uses `mrml::parse()` which only catches XML structural errors (malformed tags, unclosed elements, text in void elements). It stops at the first error and doesn't validate MJML semantics — nesting rules, required attributes, unknown tags, or singleton constraints.

## Architecture

Two validation passes run on every `didOpen` / `didChange`:

```
Source text
    |
    +-> Pass 1: Tag Scanner (new module: scanner.rs)
    |   Scans source -> Vec<TagInfo>
    |   Validates: nesting, required attrs, unknown mj-* tags, singletons
    |   Reports: ALL violations (does not stop at first)
    |
    +-> Pass 2: mrml::parse (existing)
    |   Validates: XML well-formedness, content model
    |   Reports: first structural error
    |
    +-> Merge diagnostics, deduplicate by range, publish
```

## Tag Scanner

### Data model

```rust
struct TagInfo {
    name: String,                    // "mj-section"
    tag_span: (usize, usize),       // byte range of opening tag including < and >
    self_closing: bool,
    attributes: Vec<AttrInfo>,
    parent_idx: Option<usize>,      // index into Vec<TagInfo>
}

struct AttrInfo {
    name: String,
    value: Option<String>,
    name_span: (usize, usize),      // byte range of attribute name
}
```

### Scanning strategy

1. Iterate through source tracking byte position
2. On `<`: start tag extraction
3. Skip `<!--...-->` comments entirely
4. Skip content inside `<mj-style>` (contains CSS, not MJML)
5. On `</`: record closing tag, pop parent stack
6. Extract tag name + attributes with their byte positions
7. Build parent-child relationships via a stack

### Edge cases

- Self-closing tags (`<mj-image />`) — no stack push
- Nested HTML inside `mj-text`, `mj-raw`, `mj-button` — skip non-mj-* tags (they're valid HTML content)
- `mj-style` content — skip everything until `</mj-style>`

## Validation Rules

### Rule 1: Nesting (ERROR)

Each MJML element has allowed parents. Violation message format:
`<mj-text> must be inside <mj-column>, but found inside <mj-section>`

```
mjml         -> (root, no parent)
mj-head      -> mjml
mj-body      -> mjml
mj-title     -> mj-head
mj-preview   -> mj-head
mj-style     -> mj-head
mj-font      -> mj-head
mj-breakpoint -> mj-head
mj-attributes -> mj-head
mj-html-attributes -> mj-head
mj-raw       -> mj-head, mj-column, mj-hero
mj-section   -> mj-body, mj-wrapper
mj-wrapper   -> mj-body
mj-hero      -> mj-body
mj-group     -> mj-section
mj-column    -> mj-section, mj-group
mj-text      -> mj-column, mj-hero
mj-image     -> mj-column, mj-hero
mj-button    -> mj-column, mj-hero
mj-divider   -> mj-column, mj-hero
mj-spacer    -> mj-column, mj-hero
mj-social    -> mj-column, mj-hero
mj-accordion -> mj-column, mj-hero
mj-carousel  -> mj-column, mj-hero
mj-table     -> mj-column, mj-hero
mj-navbar    -> mj-column, mj-hero
mj-social-element      -> mj-social
mj-accordion-element   -> mj-accordion
mj-accordion-title     -> mj-accordion-element
mj-accordion-text      -> mj-accordion-element
mj-carousel-image      -> mj-carousel
mj-navbar-link         -> mj-navbar
mj-all       -> mj-attributes
mj-class     -> mj-attributes
mj-selector  -> mj-html-attributes
mj-html-attribute -> mj-selector
```

Only `mj-*` tags are validated. Non-mj tags (plain HTML) inside content elements like `mj-text` are ignored — they're valid HTML content.

### Rule 2: Required attributes (WARNING)

Format: `<mj-image> is missing required attribute "src"`

```
mj-image          -> src
mj-font           -> name, href
mj-breakpoint     -> width
mj-class          -> name
mj-carousel-image -> src
mj-social-element -> name
```

### Rule 3: Unknown mj-* tags (WARNING)

Any tag starting with `mj-` that isn't in the known set.
Format: `Unknown MJML element <mj-seciton> — did you mean <mj-section>?`

Includes Levenshtein distance suggestion when edit distance <= 2.

### Rule 4: Singleton enforcement (ERROR)

`mj-head` and `mj-body` may appear at most once inside `<mjml>`.
Format: `Duplicate <mj-body> — only one <mj-body> is allowed per document`

## File structure

```
crates/mjml-lsp/src/
  main.rs        -- LSP server, calls validate_mjml()
  scanner.rs     -- Tag scanner: scan_tags(text) -> Vec<TagInfo>
  rules.rs       -- MJML spec: ALLOWED_PARENTS, REQUIRED_ATTRS, KNOWN_TAGS
  validate.rs    -- validate_tags(tags) -> Vec<Diagnostic>
```

`main.rs` is refactored: `validate_mjml()` calls `scanner::scan_tags()` + `validate::validate_tags()` + mrml parse, merges results.

## Error message principles

1. Always name the offending element with angle brackets: `<mj-text>`
2. Always explain WHY it's wrong, not just WHAT is wrong
3. Suggest the fix when possible (e.g., "did you mean?", "must be inside")
4. Show the actual parent when reporting nesting errors
