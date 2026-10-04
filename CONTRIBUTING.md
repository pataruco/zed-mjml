# Contributing

Thanks for your interest in contributing to MJML for Zed.

Keyboard shortcuts in this document are for macOS. On Linux, use `Ctrl` where `Cmd` is shown.

## Getting started

1. Fork and clone the repository
2. Install [Zed](https://zed.dev)
3. Install [Rust with `rustup`](https://rustup.rs). Zed needs a `rustup` toolchain to compile the extension, and the language server builds with the same toolchain
4. Install the extension as a dev extension:
   - Open the command palette (`Cmd+Shift+P`)
   - Run `zed: install dev extension`
   - Select the cloned directory
5. Open a `.mjml` file to test your changes

The README has more detail on [installing locally](README.md#install-locally-as-a-dev-extension).

## Project structure

```
zed-mjml/
├── extension.toml                  # Extension metadata, grammar, snippets, LSP
├── src/
│   └── lib.rs                      # WASM entry point — downloads the LSP binary from GitHub releases
├── snippets/
│   └── mjml.json                   # Element snippets (shorthand prefixes → full tags)
├── crates/
│   └── mjml-lsp/                   # MJML language server
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs             # LSP server (stdio transport, request routing, mrml pass)
│           ├── rules.rs            # MJML component registry (tags, attributes, nesting, docs)
│           ├── scanner.rs          # Lightweight tag scanner with byte positions
│           ├── completion.rs       # Tag, attribute and value completion
│           ├── hover.rs            # Tag and attribute hover documentation
│           ├── code_action.rs      # Quick fixes (turns diagnostics into edits)
│           ├── validate.rs         # Validation rules (nesting, required attributes, unknown tags, singletons)
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
│   └── invalid/                    # MJML files that should trigger errors and warnings
├── LICENSE
└── README.md
```

## How it works

The extension has two main parts:

1. Language definition (`languages/mjml/`). MJML is syntactically identical to HTML, so the extension uses [tree-sitter-html](https://github.com/tree-sitter/tree-sitter-html) to parse it. The `.scm` query files provide MJML-specific syntax highlighting, indentation and outline support.

2. Language server (`crates/mjml-lsp/`). A Rust binary that validates MJML documents in two passes:
   - Tag scanner pass: scans the source text for MJML tags and checks the semantic rules (nesting, required attributes, unknown tags, singletons)
   - mrml parser pass: uses [mrml](https://github.com/jdrouet/mrml) to catch structural XML errors such as unclosed tags and malformed markup. `mj-include` paths are resolved relative to the document

   A file without an `<mjml>` root is treated as an `mj-include` partial: its root-level tags skip the nesting rule and the mrml pass is skipped, because MJML grafts such fragments into the including document.

## Making changes

### Syntax highlighting

Edit `languages/mjml/highlights.scm`. Capture names map to theme colours:

- `@keyword`: structural tags (`mjml`, `mj-head`, `mj-body`)
- `@type`: head configuration tags such as `mj-attributes` and `mj-style`
- `@tag`: all other tags
- `@attribute`: attribute names
- `@string`: attribute values and quotes
- `@comment`: HTML comments

### Adding new MJML tags

If MJML adds a component:

1. Add a `ComponentSpec` entry to `COMPONENTS` in `crates/mjml-lsp/src/rules.rs`, with its allowed parents, attributes and documentation link. Validation, completion and hover all read from this table
2. Update the `#match?` patterns in `languages/mjml/highlights.scm` so the tag is categorised correctly

### Language injection

Edit `languages/mjml/injections.scm` to add or change embedded language support, such as CSS in `<mj-style>` or JavaScript in `<script>`.

### LSP and diagnostics

The validation logic is split across three modules in `crates/mjml-lsp/src/`:

- `rules.rs`: MJML specification data (known tags, allowed parents, required attributes, typo suggestions by Levenshtein distance)
- `scanner.rs`: byte-level tag scanner that extracts `TagInfo` structs with attributes and parent-child relationships
- `validate.rs`: walks the scanned tags and produces `LintDiagnostic` results for nesting, required attributes, unknown tags and singletons. Fixable diagnostics (unknown tag, missing required attribute) also carry an optional `LintFix`

Quick fixes are handled separately:

- `code_action.rs`: turns the `LintFix` embedded in a diagnostic into a `WorkspaceEdit` when the editor requests a code action

## Testing

### Automated tests

Run the LSP test suite:

```bash
cargo test --manifest-path crates/mjml-lsp/Cargo.toml
```

### Manual testing in Zed

The `test/` folder contains sample MJML files. Each file starts with a comment stating what it covers and what diagnostics to expect:

```
test/
├── valid/        — Files that should show no diagnostics
│   ├── attributes.mjml     — Component tags as defaults inside mj-attributes
│   ├── default.mjml
│   ├── full.mjml
│   ├── head-only.mjml
│   ├── minimal.mjml
│   ├── with-include.mjml   — Template that includes the partials below
│   └── partials/
│       ├── head.mjml       — Head partial without an <mjml> root
│       └── body.mjml       — Body partial without an <mjml> root
└── invalid/      — Files that should trigger errors and warnings
    ├── default.mjml        — Exercises every validation rule
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

After changing the LSP:

1. Rebuild: `cargo build --manifest-path crates/mjml-lsp/Cargo.toml`
2. Restart Zed (`Cmd+Q`) to pick up the new binary
3. Open files from `test/valid/` and check that no diagnostics appear
4. Open files from `test/invalid/` and check that the expected errors and warnings are highlighted

After changing the language definition (`.scm` files), reload the extension instead:

1. Open the command palette (`Cmd+Shift+P`)
2. Run `zed: reload extensions`

## Submitting changes

1. Create a branch for your changes
2. Run `cargo test --manifest-path crates/mjml-lsp/Cargo.toml` and make sure all tests pass
3. Test manually with files in the `test/` folder
4. Open a pull request with a clear description of what changed and why

## Releasing

Releases are managed with [release-please](https://github.com/googleapis/release-please) plus a manual binary build. The version in `Cargo.toml`, `extension.toml`, `crates/mjml-lsp/Cargo.toml` and both `Cargo.lock` files is kept in sync automatically, and it must match the version published to the Zed registry.

1. Land changes on `main` using [Conventional Commits](https://www.conventionalcommits.org) (`feat:`, `fix:` and so on). These determine the next version number.

2. Merge the release-please PR. release-please opens and continuously updates a "release" pull request that bumps the version in every file above and updates `CHANGELOG.md`. Merging it creates the `zed-mjml-v<version>` tag and a matching GitHub release.

   The lock file entry is targeted in `release-please-config.json` with the JSONPath `$.package[?(@.name.value=="mjml-lsp")].version`. The `.value` is deliberate: release-please parses TOML into tagged values (`{value, start, end}`) so it can edit in place, so the filter has to compare against that inner field.

3. Build and upload the language server binaries. From the Actions tab, run the Deploy workflow (`.github/workflows/deploy.yaml`) and pass the new tag, for example `zed-mjml-v0.3.0`. It cross-compiles `mjml-lsp` and uploads one `mjml-lsp-<target>.gz` asset per platform to the release:
   - `aarch64-apple-darwin`
   - `x86_64-apple-darwin`
   - `x86_64-unknown-linux-gnu`

   This step is required: `src/lib.rs` downloads these assets from the latest GitHub release at install time, so the release must carry them before anyone installs the new version.

4. Update the Zed extension registry. Open a pull request against [`zed-industries/extensions`](https://github.com/zed-industries/extensions):
   - Update the `extensions/mjml` submodule to the released commit
   - Set the `version` for `[mjml]` in `extensions.toml` to match `extension.toml`
   - Run `pnpm sort-extensions` to keep `extensions.toml` and `.gitmodules` sorted

   Once the PR is merged, Zed packages and publishes the new version.

> To automate step 4 later, the community [`huacnlee/zed-extension-action`](https://github.com/huacnlee/zed-extension-action) can open the registry PR for you on tag push.

## Resources

- [Zed extension documentation](https://zed.dev/docs/extensions)
- [Zed language extensions](https://zed.dev/docs/extensions/languages)
- [Tree-sitter query syntax](https://tree-sitter.github.io/tree-sitter/using-parsers/queries)
- [MJML documentation](https://documentation.mjml.io/)
- [MJML components](https://mjml.io/components)
- [mrml (Rust MJML parser)](https://github.com/jdrouet/mrml)
