# MJML for Zed

A [Zed](https://zed.dev) extension that adds language support for [MJML](https://mjml.io), the email markup language.

[![Zed Extension](https://img.shields.io/badge/Zed-Extension-084CCF)](https://zed.dev/extensions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Keyboard shortcuts in this document are for macOS. On Linux, use `Ctrl` where `Cmd` is shown.

## Features

- Syntax highlighting: structural tags (`mjml`, `mj-head`, `mj-body`) are visually distinct from layout and content tags
- Bracket matching: jump between opening and closing MJML tags
- Auto-indentation: smart indentation for nested MJML elements
- Comment toggling: `Cmd+/` toggles `<!-- -->` HTML comments
- Document outline: `Cmd+Shift+O` navigates the MJML structure
- CSS injection: syntax highlighting for CSS inside `<mj-style>` blocks and inline `style` attributes
- Word-aware navigation: hyphenated tag names like `mj-section` are treated as single words for selection and navigation
- Diagnostics: real-time error reporting from the built-in MJML language server (powered by [mrml](https://github.com/jdrouet/mrml)):
  - Nesting: reports elements placed inside the wrong parent, for example `<mj-text>` directly inside `<mj-section>`
  - Partials and defaults: files without an `<mjml>` root are treated as `mj-include` partials, and component tags inside `<mj-attributes>` are treated as default declarations, so neither is reported as a nesting error
  - Includes: `<mj-include path="...">` is resolved relative to the current file, so a template split into partials validates as a whole
  - Required attributes: warns about missing required attributes, for example `src` on `<mj-image>`
  - Unknown tags: flags unknown `mj-*` elements with "did you mean?" suggestions for typos
  - Singletons: errors on duplicate `<mj-head>` or `<mj-body>` elements
  - Structural errors: reports XML syntax errors, unclosed tags and missing root elements
- Quick fixes: one-click code actions that replace an unknown `mj-*` tag with its suggested correction or insert a missing required attribute
- Completions: context-aware suggestions for tags (valid children ranked first), attributes and enumerated attribute values
- Hover documentation: component and attribute docs with a link to the MJML reference
- Snippets: shorthand prefixes like `mjsection`, `mjimage` and `mjml` expand to full MJML elements with tab stops

## Installation

### Install in Zed (from the extension registry)

MJML is published in the official Zed extension registry, so you can install it directly from the editor:

1. Open Zed
2. Open the Extensions panel: press `Cmd+Shift+X`, or run `zed: extensions` from the command palette (`Cmd+Shift+P`)
3. Search for MJML
4. Click Install

Syntax highlighting, indentation and the document outline work immediately. The MJML language server that powers diagnostics is downloaded automatically the first time you open a `.mjml` file, with no extra setup.

Prebuilt language server binaries are provided for macOS (Apple Silicon and Intel) and Linux (x86-64). Windows is not supported yet: the extension installs, but diagnostics will not work.

### Install locally (as a dev extension)

#### Prerequisites

- [Zed](https://zed.dev)
- [Rust, installed with `rustup`](https://rustup.rs). Zed compiles the extension to WebAssembly when you install it, and a Rust toolchain installed another way (for example with Homebrew) will not work for dev extensions.

#### Steps

1. Clone this repository:

   ```bash
   git clone https://github.com/pataruco/zed-mjml.git
   ```

2. In Zed, open the command palette (`Cmd+Shift+P`)
3. Run `zed: install dev extension`
4. Select the cloned directory

Zed builds the extension locally and loads it. As with the registry build, the language server binary is downloaded from the latest GitHub release the first time you open a `.mjml` file. If you already have the published version installed, Zed replaces it with your dev build, shown as "Overridden by dev extension" in the Extensions panel.

## Supported tags

All standard MJML components are supported:

| Category    | Tags                                                                                                                          |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Root        | `mjml`, `mj-head`, `mj-body`, `mj-include`                                                                                    |
| Head        | `mj-attributes`, `mj-all`, `mj-class`, `mj-breakpoint`, `mj-font`, `mj-html-attributes`, `mj-preview`, `mj-style`, `mj-title` |
| Layout      | `mj-section`, `mj-column`, `mj-group`, `mj-wrapper`, `mj-hero`                                                                |
| Content     | `mj-text`, `mj-button`, `mj-image`, `mj-divider`, `mj-spacer`, `mj-table`, `mj-raw`                                           |
| Interactive | `mj-accordion`, `mj-carousel`, `mj-navbar`, `mj-social`                                                                       |

## Snippets

Type a shorthand prefix and accept the completion to expand a full MJML element with tab stops. Prefixes follow an `mj<tag>` convention, with no hyphen:

| Prefix      | Expands to                                                                     |
| ----------- | ------------------------------------------------------------------------------ |
| `mjml`      | A complete document skeleton (`mjml` → `mj-body` → `mj-section` → `mj-column`) |
| `mjsection` | `<mj-section>` wrapping an `<mj-column>`                                       |
| `mjcolumn`  | `<mj-column>`                                                                  |
| `mjimage`   | `<mj-image src="" alt="" />`                                                   |
| `mjbutton`  | `<mj-button href="">…</mj-button>`                                             |
| `mjtext`    | `<mj-text>`                                                                    |

All common components have a snippet. See [`snippets/mjml.json`](snippets/mjml.json) for the full list.

## How it works

MJML is syntactically identical to HTML with custom element names, so this extension reuses the [tree-sitter-html](https://github.com/tree-sitter/tree-sitter-html) grammar. Tree-sitter query files provide MJML-specific syntax highlighting, indentation and document outline support. Diagnostics, completions, hover and quick fixes come from a small Rust language server in this repository.

## Testing

Sample MJML files for manual testing live in `test/`. See [Testing in CONTRIBUTING.md](CONTRIBUTING.md#testing) for what each file covers and how to use them.

## License

[MIT](LICENSE)
