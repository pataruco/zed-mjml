# Contributing

Thanks for your interest in contributing to MJML for Zed!

## Getting Started

1. Fork and clone the repository
2. Install [Zed](https://zed.dev)
3. Install the extension locally:
   - Open the command palette (`Cmd+Shift+P`)
   - Run "zed: install dev extension"
   - Select the cloned directory
4. Open a `.mjml` file to test your changes

## Project Structure

```
zed-mjml/
├── extension.toml                  # Extension metadata and grammar reference
├── languages/
│   └── mjml/
│       ├── config.toml             # Language configuration
│       ├── highlights.scm          # Syntax highlighting queries
│       ├── brackets.scm            # Bracket matching
│       ├── indents.scm             # Auto-indentation rules
│       ├── outline.scm             # Document outline navigation
│       ├── injections.scm          # CSS/JS language injection
│       └── overrides.scm           # Scope overrides
├── LICENSE
└── README.md
```

## How It Works

This extension uses [tree-sitter-html](https://github.com/tree-sitter/tree-sitter-html) to parse MJML files since MJML is syntactically identical to HTML. The `.scm` query files in `languages/mjml/` provide MJML-specific behavior on top of the HTML grammar.

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

If MJML adds new components, update the `#match?` patterns in `highlights.scm` to categorise them appropriately.

### Language Injection

Edit `languages/mjml/injections.scm` to add or modify embedded language support (CSS in `<mj-style>`, JavaScript in `<script>`, etc.).

## Testing

After making changes, reload the extension in Zed:

1. Open the command palette (`Cmd+Shift+P`)
2. Run "zed: reload extensions"
3. Open a `.mjml` file and verify your changes

Use `test.mjml` in the repository root as a comprehensive test file covering all MJML components.

## Submitting Changes

1. Create a branch for your changes
2. Test thoroughly with various MJML templates
3. Open a pull request with a clear description of what changed and why

## Resources

- [Zed Extension Documentation](https://zed.dev/docs/extensions)
- [Zed Language Extensions](https://zed.dev/docs/extensions/languages)
- [Tree-sitter Query Syntax](https://tree-sitter.github.io/tree-sitter/using-parsers/queries)
- [MJML Documentation](https://documentation.mjml.io/)
- [MJML Components](https://mjml.io/components)
