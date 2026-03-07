# Contributing

Thanks for your interest in contributing to MJML for Zed!

## Getting Started

1. Fork and clone the repository
2. Install [Zed](https://zed.dev)
3. Install [Rust](https://rustup.rs) (for building the LSP)
4. Install the extension locally:
   - Open the command palette (`Cmd+Shift+P`)
   - Run "zed: install dev extension"
   - Select the cloned directory
5. Open a `.mjml` file to test your changes

## Project Structure

```
zed-mjml/
├── extension.toml                  # Extension metadata and grammar reference
├── src/
│   └── lib.rs                      # WASM entry point — downloads LSP binary from GitHub releases
├── crates/
│   └── mjml-lsp/                   # MJML language server
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs             # LSP server (stdio transport, diagnostics)
│           ├── rules.rs            # MJML spec data (known tags, nesting, required attrs)
│           ├── scanner.rs          # Lightweight tag scanner with byte positions
│           ├── validate.rs         # Validation rules engine (4 rules)
│           └── tests.rs            # Integration tests for the LSP
├── languages/
│   └── mjml/
│       ├── config.toml             # Language configuration
│       ├── highlights.scm          # Syntax highlighting queries
│       ├── brackets.scm            # Bracket matching
│       ├── indents.scm             # Auto-indentation rules
│       ├── outline.scm             # Document outline navigation
│       ├── injections.scm          # CSS/JS language injection
│       └── overrides.scm           # Scope overrides
├── test/
│   ├── valid/                      # MJML files that should show no diagnostics
│   └── invalid/                    # MJML files that should trigger errors/warnings
├── LICENSE
└── README.md
```

## How It Works

The extension has two main parts:

1. **Language definition** (`languages/mjml/`) — Uses [tree-sitter-html](https://github.com/tree-sitter/tree-sitter-html) to parse MJML files since MJML is syntactically identical to HTML. The `.scm` query files provide MJML-specific syntax highlighting, indentation, and outline support.

2. **Language server** (`crates/mjml-lsp/`) — A Rust binary that validates MJML documents using two passes:
   - **Tag scanner pass** — Scans source text for MJML tags and validates semantic rules (nesting, required attributes, unknown tags, singletons)
   - **mrml parser pass** — Uses [mrml](https://github.com/jdrouet/mrml) to catch structural XML errors (unclosed tags, malformed markup)

## Making Changes

### Syntax Highlighting

Edit `languages/mjml/highlights.scm`. Capture names map to theme colours:

- `@keyword` — structural tags (`mjml`, `mj-head`, `mj-body`)
- `@type` — head configuration tags (`mj-attributes`, `mj-style`, etc.)
- `@tag` — all other tags
- `@attribute` — attribute names
- `@string` — attribute values and quotes
- `@comment` — HTML comments

### Adding New MJML Tags

If MJML adds new components:

1. Add the tag to `KNOWN_TAGS` in `crates/mjml-lsp/src/rules.rs`
2. Add nesting rules in `allowed_parents()`
3. Add required attributes in `required_attributes()` (if any)
4. Update the `#match?` patterns in `languages/mjml/highlights.scm` to categorise them appropriately

### Language Injection

Edit `languages/mjml/injections.scm` to add or modify embedded language support (CSS in `<mj-style>`, JavaScript in `<script>`, etc.).

### LSP / Diagnostics

The validation logic is split across three modules in `crates/mjml-lsp/src/`:

- **`rules.rs`** — MJML specification data (known tags, allowed parents, required attributes, typo suggestions via Levenshtein distance)
- **`scanner.rs`** — Byte-level tag scanner that extracts `TagInfo` structs with attributes and parent-child relationships
- **`validate.rs`** — Walks scanned tags and produces `LintDiagnostic` results for 4 rules: nesting, required attributes, unknown tags, and singleton enforcement

## Testing

### Automated Tests

Run the LSP test suite:

```bash
cargo test --manifest-path crates/mjml-lsp/Cargo.toml
```

### Manual Testing in Zed

The `test/` folder contains sample MJML files for manual verification:

- `test/valid/` — Files that should show no diagnostics
- `test/invalid/` — Files that should trigger specific errors and warnings

After making changes to the LSP:

1. Rebuild: `cargo build --manifest-path crates/mjml-lsp/Cargo.toml`
2. Restart Zed (`Cmd+Q`) to pick up the new binary
3. Open files from `test/valid/` and `test/invalid/` to verify

For language definition changes (`.scm` files), reload the extension:

1. Open the command palette (`Cmd+Shift+P`)
2. Run "zed: reload extensions"

## Submitting Changes

1. Create a branch for your changes
2. Run `cargo test --manifest-path crates/mjml-lsp/Cargo.toml` and ensure all tests pass
3. Test manually with files in the `test/` folder
4. Open a pull request with a clear description of what changed and why

## Resources

- [Zed Extension Documentation](https://zed.dev/docs/extensions)
- [Zed Language Extensions](https://zed.dev/docs/extensions/languages)
- [Tree-sitter Query Syntax](https://tree-sitter.github.io/tree-sitter/using-parsers/queries)
- [MJML Documentation](https://documentation.mjml.io/)
- [MJML Components](https://mjml.io/components)
- [mrml (Rust MJML parser)](https://github.com/jdrouet/mrml)
