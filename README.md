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
- Diagnostics — Real-time error reporting via the built-in MJML language server (powered by [mrml](https://github.com/jdrouet/mrml))

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

## How It Works

This extension reuses the [tree-sitter-html](https://github.com/tree-sitter/tree-sitter-html) grammar since MJML is syntactically identical to HTML with custom element names. Tree-sitter query files provide MJML-specific syntax highlighting, indentation, and document outline support.

## License

[MIT](LICENSE)
