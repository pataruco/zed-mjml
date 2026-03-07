# MJML for Zed

A [Zed](https://zed.dev) extension that adds language support for [MJML](https://mjml.io), the email markup language.

## Features

- Syntax highlighting — MJML structural tags (`mjml`, `mj-head`, `mj-body`) are visually distinct from layout and content tags
- Bracket matching — Navigate between opening and closing MJML tags.
- Auto-indentation — Smart indentation for nested MJML elements
- Comment toggling — `Cmd+/` toggles `<!-- -->` HTML comments
- Document outline — `Cmd+Shift+O` to navigate MJML structure
- CSS injection — Syntax highlighting for CSS inside `<mj-style>` blocks and inline `style` attributes
- Word-aware navigation — Hyphenated tag names like `mj-section` are treated as single words for selection and navigation
- Diagnostics — Real-time error reporting via the built-in MJML language server (powered by [mrml](https://github.com/jdrouet/mrml)):
  - **Nesting validation** — Reports when MJML elements are placed inside incorrect parents (e.g. `<mj-text>` directly inside `<mj-section>`)
  - **Required attributes** — Warns about missing required attributes (e.g. `src` on `<mj-image>`)
  - **Unknown tag detection** — Flags unknown `mj-*` elements with "did you mean?" suggestions for typos
  - **Singleton enforcement** — Errors on duplicate `<mj-head>` or `<mj-body>` elements
  - **Structural errors** — Reports XML syntax errors, unclosed tags, and missing root elements

## Supported Tags

All standard MJML components are supported:

| Category | Tags |
|----------|------|
| Root | `mjml`, `mj-head`, `mj-body`, `mj-include` |
| Head | `mj-attributes`, `mj-all`, `mj-class`, `mj-breakpoint`, `mj-font`, `mj-html-attributes`, `mj-preview`, `mj-style`, `mj-title` |
| Layout | `mj-section`, `mj-column`, `mj-group`, `mj-wrapper` |
| Content | `mj-text`, `mj-button`, `mj-image`, `mj-divider`, `mj-spacer`, `mj-table`, `mj-raw` |
| Interactive | `mj-accordion`, `mj-carousel`, `mj-navbar`, `mj-social`, `mj-hero` |

## Installation

### From the Extension Registry

1. Open Zed
2. Open the Extensions panel (`Cmd+Shift+X`)
3. Search for "MJML"
4. Click Install

### As a Dev Extension

1. Clone this repository
2. In Zed, open the command palette (`Cmd+Shift+P`)
3. Run "zed: install dev extension"
4. Select the cloned directory

## Testing Locally

The `test/` folder contains sample MJML files for manually verifying the extension in Zed:

```
test/
├── valid/        — Files that should show no diagnostics
│   ├── default.mjml
│   ├── full.mjml
│   ├── head-only.mjml
│   └── minimal.mjml
└── invalid/      — Files that should trigger errors and warnings
    ├── default.mjml        — Exercises all 4 validation rules
    ├── nesting.mjml        — Nesting violations
    ├── required-attrs.mjml — Missing required attributes
    ├── unknown-tags.mjml   — Typos with "did you mean?" suggestions
    ├── singletons.mjml     — Duplicate mj-head/mj-body
    ├── combined.mjml       — Multiple rule violations combined
    ├── bad-xml.mjml        — Malformed XML
    ├── empty.mjml          — Empty file
    ├── no-root.mjml        — Missing <mjml> root
    ├── text-in-image.mjml  — Text inside void element
    └── unclosed-tag.mjml   — Unclosed tags
```

To test:

1. Install the extension as a dev extension (see [Installation](#as-a-dev-extension))
2. Open any file from `test/valid/` — verify no diagnostics appear
3. Open any file from `test/invalid/` — verify errors/warnings are highlighted
4. After making changes to the LSP, rebuild with `cargo build --manifest-path crates/mjml-lsp/Cargo.toml` and restart Zed (`Cmd+Q`) to pick up the new binary

## How It Works

This extension reuses the [tree-sitter-html](https://github.com/tree-sitter/tree-sitter-html) grammar since MJML is syntactically identical to HTML with custom element names. Tree-sitter query files provide MJML-specific syntax highlighting, indentation, and document outline support.

## License

[MIT](LICENSE)
