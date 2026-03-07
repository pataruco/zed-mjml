# MJML LSP Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a diagnostics-only MJML Language Server using mrml, integrated into the zed-mjml Zed extension.

**Architecture:** Two independent Cargo projects in one repo — root is the Zed extension (cdylib → WASM), `crates/mjml-lsp/` is the LSP binary (native). The extension downloads the LSP binary from GitHub releases and launches it. The LSP parses MJML with `mrml::parse()` and sends diagnostics back to Zed.

**Tech Stack:** Rust, mrml 5.x, lsp-server 0.7, lsp-types 0.97, zed_extension_api 0.7

---

### Task 1: Create the mjml-lsp binary crate

**Files:**
- Create: `crates/mjml-lsp/Cargo.toml`
- Create: `crates/mjml-lsp/src/main.rs`

**Step 1: Create Cargo.toml for the LSP binary**

```toml
[package]
name = "mjml-lsp"
version = "0.1.0"
edition = "2021"
license = "MIT"

[[bin]]
name = "mjml-lsp"
path = "src/main.rs"

[dependencies]
mrml = "5"
lsp-server = "0.7"
lsp-types = "0.97"
serde_json = "1"
```

**Step 2: Create a minimal main.rs that compiles**

```rust
use std::error::Error;

use lsp_server::{Connection, Message, Notification};
use lsp_types::{
    InitializeParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
};

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("mjml-lsp: starting");

    let (connection, _init_params) = Connection::stdio();
    let capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),
        ..Default::default()
    })?;
    connection.initialize(capabilities)?;

    eprintln!("mjml-lsp: initialized");

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
            }
            Message::Notification(_notif) => {}
            Message::Response(_) => {}
        }
    }

    Ok(())
}
```

**Step 3: Build to verify it compiles**

Run: `cd crates/mjml-lsp && cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add crates/mjml-lsp/
git commit -m "feat: scaffold mjml-lsp binary crate"
```

---

### Task 2: Add diagnostic publishing on document open/change

**Files:**
- Modify: `crates/mjml-lsp/src/main.rs`

**Step 1: Implement document validation and diagnostic publishing**

Replace the notification handler in main.rs:

```rust
use std::error::Error;

use lsp_server::{Connection, Message, Notification};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, Position, PublishDiagnosticsParams, Range, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("mjml-lsp: starting");

    let (connection, _init_params) = Connection::stdio();
    let capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),
        ..Default::default()
    })?;
    connection.initialize(capabilities)?;

    eprintln!("mjml-lsp: initialized");

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
            }
            Message::Notification(notif) => {
                handle_notification(&connection, notif)?;
            }
            Message::Response(_) => {}
        }
    }

    Ok(())
}

fn handle_notification(
    connection: &Connection,
    notif: Notification,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    match notif.method.as_str() {
        "textDocument/didOpen" => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(notif.params)?;
            let uri = params.text_document.uri;
            let text = params.text_document.text;
            validate_and_publish(connection, uri, &text)?;
        }
        "textDocument/didChange" => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(notif.params)?;
            let uri = params.text_document.uri;
            if let Some(change) = params.content_changes.into_iter().last() {
                validate_and_publish(connection, uri, &change.text)?;
            }
        }
        "textDocument/didClose" => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(notif.params)?;
            publish_diagnostics(connection, params.text_document.uri, vec![])?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_and_publish(
    connection: &Connection,
    uri: Uri,
    text: &str,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let diagnostics = validate_mjml(text);
    publish_diagnostics(connection, uri, diagnostics)?;
    Ok(())
}

fn validate_mjml(text: &str) -> Vec<Diagnostic> {
    match mrml::parse(text) {
        Ok(_) => vec![],
        Err(err) => {
            let message = format!("{err}");
            vec![Diagnostic {
                range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("mjml".to_string()),
                message,
                ..Default::default()
            }]
        }
    }
}

fn publish_diagnostics(
    connection: &Connection,
    uri: Uri,
    diagnostics: Vec<Diagnostic>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let params = PublishDiagnosticsParams::new(uri, diagnostics, None);
    let notif = Notification::new(
        "textDocument/publishDiagnostics".to_string(),
        serde_json::to_value(params)?,
    );
    connection.sender.send(Message::Notification(notif))?;
    Ok(())
}
```

**Step 2: Build to verify**

Run: `cd crates/mjml-lsp && cargo build`
Expected: Compiles successfully

**Step 3: Quick smoke test via stdin/stdout**

Run: `cd crates/mjml-lsp && cargo build && echo "LSP binary built at: target/debug/mjml-lsp"`
Expected: Binary built successfully

**Step 4: Commit**

```bash
git add crates/mjml-lsp/src/main.rs
git commit -m "feat: add MJML validation and diagnostic publishing"
```

---

### Task 3: Improve error position extraction

**Files:**
- Modify: `crates/mjml-lsp/src/main.rs`

mrml parse errors may include position information. We need to extract line/column from the error to highlight the correct location.

**Step 1: Investigate mrml error format**

Run: `cd crates/mjml-lsp && cargo doc --open -p mrml`

Look at the `mrml::prelude::parser::Error` type to see if it has `Span` or position fields.

**Step 2: Update validate_mjml to extract position**

If mrml errors include position info, map them. If not, compute line/column by counting newlines up to the error position. Update `validate_mjml` accordingly:

```rust
fn error_position(text: &str, byte_offset: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;
    for (i, ch) in text.char_indices() {
        if i >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    Position::new(line, character)
}
```

Integrate this helper into `validate_mjml` based on what mrml exposes.

**Step 3: Build and test**

Run: `cd crates/mjml-lsp && cargo build`
Expected: Compiles

**Step 4: Commit**

```bash
git add crates/mjml-lsp/src/main.rs
git commit -m "feat: extract error positions for accurate diagnostics"
```

---

### Task 4: Create the Zed extension WASM entry point

**Files:**
- Create: `Cargo.toml` (root — extension cdylib)
- Create: `src/lib.rs`
- Modify: `extension.toml`

**Step 1: Create root Cargo.toml for the Zed extension**

```toml
[package]
name = "zed-mjml"
version = "0.1.0"
edition = "2021"
license = "MIT"

[lib]
crate-type = ["cdylib"]

[dependencies]
zed_extension_api = "0.7"
```

**Step 2: Update extension.toml to declare the language server**

Add `[language_servers.mjml-lsp]` section:

```toml
id = "mjml"
name = "MJML"
description = "MJML email markup language support for Zed."
version = "0.1.0"
schema_version = 1
authors = ["Pedro Martin <pataruco@gmail.com>"]
repository = "https://github.com/pataruco/zed-mjml"

[grammars.html]
repository = "https://github.com/tree-sitter/tree-sitter-html"
commit = "bfa075d83c6b97cd48440b3829ab8d24a2319809"

[language_servers.mjml-lsp]
name = "MJML LSP"
languages = ["MJML"]
```

**Step 3: Create src/lib.rs — extension entry point**

```rust
use std::fs;

use zed_extension_api::{self as zed, LanguageServerId, Result};

struct MjmlExtension {
    cached_binary_path: Option<String>,
}

impl MjmlExtension {
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |m| m.is_file()) {
                return Ok(path.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            "pataruco",
            "zed-mjml",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();
        let asset_name = format!(
            "mjml-lsp-{}-{}.gz",
            match platform {
                zed::Os::Mac => "apple-darwin",
                zed::Os::Linux => "unknown-linux-gnu",
                zed::Os::Windows => "pc-windows-msvc",
            },
            match arch {
                zed::Architecture::Aarch64 => "aarch64",
                zed::Architecture::X8664 => "x86_64",
                zed::Architecture::X86 => "x86",
            },
        );

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {asset_name}"))?;

        let binary_path = format!("mjml-lsp");

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        zed::download_file(
            &asset.download_url,
            &binary_path,
            zed::DownloadedFileType::Gzip,
        )
        .map_err(|e| format!("failed to download mjml-lsp: {e}"))?;

        zed::make_file_executable(&binary_path)?;

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for MjmlExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary_path = self.language_server_binary_path(language_server_id)?;
        Ok(zed::Command {
            command: binary_path,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(MjmlExtension);
```

**Step 4: Verify the extension compiles (requires wasm target)**

Run: `rustup target add wasm32-wasip2 && cargo build --target wasm32-wasip2`
Expected: Compiles (or may need adjustments based on zed_extension_api version)

**Step 5: Commit**

```bash
git add Cargo.toml src/lib.rs extension.toml
git commit -m "feat: add Zed extension WASM entry point with LSP download"
```

---

### Task 5: Add .gitignore entries and update docs

**Files:**
- Modify: `.gitignore`
- Modify: `README.md`

**Step 1: Update .gitignore**

Add Rust build artifacts:

```
grammars/
target/
```

**Step 2: Update README.md**

Add a section about diagnostics/linting to the Features list in README.md.

**Step 3: Commit**

```bash
git add .gitignore README.md
git commit -m "docs: update gitignore and README for LSP"
```

---

### Task 6: Set up CI for building and releasing the LSP binary

**Files:**
- Create: `.github/workflows/release.yml`

**Step 1: Create GitHub Actions workflow**

This workflow triggers on git tags (`v*`), builds the LSP binary for macOS (arm64, x86_64) and Linux (x86_64), compresses each with gzip, and uploads them as release assets.

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build mjml-lsp
        run: cargo build --release --manifest-path crates/mjml-lsp/Cargo.toml --target ${{ matrix.target }}

      - name: Package binary
        run: |
          cd crates/mjml-lsp/target/${{ matrix.target }}/release
          gzip -c mjml-lsp > mjml-lsp-${{ matrix.target }}.gz

      - name: Upload release asset
        uses: softprops/action-gh-release@v2
        with:
          files: crates/mjml-lsp/target/${{ matrix.target }}/release/mjml-lsp-${{ matrix.target }}.gz
```

**Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add release workflow for mjml-lsp binaries"
```

---

### Task 7: Local end-to-end test

**Step 1: Build the LSP binary locally**

Run: `cd crates/mjml-lsp && cargo build`

**Step 2: Reinstall the dev extension in Zed**

In Zed: `Cmd+Shift+P` → "zed: install dev extension" → select `/Users/pataruco/dev/zed-mjml`

Note: For local dev testing, temporarily modify `src/lib.rs` to point to the locally built binary instead of downloading from GitHub. Or set the LSP binary path in Zed settings.

**Step 3: Open test.mjml and introduce an error**

Edit `test.mjml` to have invalid MJML (e.g. unclosed tag, unknown component) and verify that error diagnostics appear in the editor.

**Step 4: Fix the error and verify diagnostics clear**

Remove the invalid MJML and verify the error indicators disappear.

---

### Summary

| Task | What | Key Files |
|------|------|-----------|
| 1 | Scaffold LSP crate | `crates/mjml-lsp/Cargo.toml`, `main.rs` |
| 2 | Validation + diagnostics | `main.rs` (handle notifications, parse, publish) |
| 3 | Error position extraction | `main.rs` (line/col from mrml errors) |
| 4 | Zed extension WASM | `Cargo.toml`, `src/lib.rs`, `extension.toml` |
| 5 | Docs + gitignore | `.gitignore`, `README.md` |
| 6 | CI release workflow | `.github/workflows/release.yml` |
| 7 | End-to-end test | Manual verification in Zed |
