# MJML LSP Design

## Context

The zed-mjml extension provides syntax highlighting, bracket matching, indentation, and document outline for MJML files. To add diagnostics (error reporting for invalid MJML), we need a Language Server Protocol (LSP) implementation. No MJML LSP exists anywhere — this will be the first.

## Decisions

- **Scope:** Diagnostics only (Phase 1), with attribute/nesting validation added iteratively (Phase 2)
- **Language:** Rust — matches the Zed extension ecosystem, single binary, fast
- **Parser:** `mrml` crate — Rust MJML parser, handles structural validation
- **LSP framework:** `lsp-server` + `lsp-types` — same minimal stack as rust-analyzer
- **Repo structure:** Monorepo — LSP lives in `crates/mjml-lsp/` alongside the Zed extension

## Architecture

```
zed-mjml/
├── extension.toml              # declares [language_servers.mjml-lsp]
├── Cargo.toml                  # workspace: members = ["crates/mjml-lsp"]
├── src/lib.rs                  # Zed extension WASM — downloads + launches LSP
├── crates/
│   └── mjml-lsp/
│       ├── Cargo.toml          # binary: mrml, lsp-server, lsp-types, serde_json
│       └── src/
│           ├── main.rs         # LSP server: init, message loop, diagnostics
│           └── validator.rs    # MJML validation rules (Phase 2)
├── languages/mjml/             # existing tree-sitter query files
└── ...
```

## Data Flow

1. User opens/edits `.mjml` file in Zed
2. Zed extension launches `mjml-lsp` binary
3. LSP receives `textDocument/didOpen` / `textDocument/didChange`
4. LSP runs validation pipeline:
   - Phase 1: `mrml::parse()` — syntax errors, unknown tags, malformed structure
   - Phase 2: Custom validator — allowed attributes, nesting rules, required attributes
5. LSP sends `textDocument/publishDiagnostics` with errors/warnings

## LSP Methods (Phase 1)

| Method | Behaviour |
|--------|-----------|
| `initialize` | Return capabilities: `textDocumentSync = Full`, diagnostics |
| `textDocument/didOpen` | Parse document, publish diagnostics |
| `textDocument/didChange` | Re-parse, publish updated diagnostics |
| `textDocument/didClose` | Clear diagnostics |

## Phase 2: Custom Validation Rules

A `validator.rs` module encoding MJML component specs:

- **Allowed attributes** per component (e.g. `mj-image` allows `src`, `alt`, `width`, etc.)
- **Required attributes** (e.g. `mj-image` requires `src`)
- **Allowed children** (e.g. `mj-section` allows `mj-column`, `mj-group`)
- **Nesting rules** (e.g. `mj-column` cannot contain `mj-section`)

Walk the mrml AST after successful parsing. Emit `Warning` severity diagnostics.

## Distribution

The Zed extension (`src/lib.rs`) handles binary distribution:

1. On `language_server_command()`, check if `mjml-lsp` binary exists in extension working directory
2. If missing, download from GitHub releases for the user's platform (macOS arm64/x86, Linux x86)
3. Return binary path to Zed

## Future Enhancements

- Completions: autocomplete tag names and attributes
- Hover: show attribute documentation
- Go-to-definition: resolve `mj-include` paths
