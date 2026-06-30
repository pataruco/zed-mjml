// MJML-to-HTML rendering and browser-preview helpers.
//
// Two groups of building blocks for the "Open Preview in Browser" code action:
// rendering (MJML to standalone HTML, error page, browser-open command, temp
// file) and the marker-pulse trigger (the only way to detect that a user
// selected a code action in Zed, which has no workspace/executeCommand). The
// LSP message glue lives in main.rs.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use lsp_server::Request as ServerRequest;
use lsp_types::{
    ApplyWorkspaceEditParams, CodeAction, CodeActionKind, CodeActionOrCommand, Position, Range,
    TextEdit, Uri, WorkspaceEdit,
};

/// The operating system a browser-open command is being built for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetOs {
    Mac,
    Linux,
    Windows,
}

/// Why rendering an MJML document to HTML failed.
#[derive(Debug)]
pub enum RenderError {
    /// The document could not be parsed as MJML.
    Parse(String),
    /// The document parsed but could not be rendered to HTML.
    Render(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "Failed to parse MJML: {msg}"),
            Self::Render(msg) => write!(f, "Failed to render MJML: {msg}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Renders an MJML document to a standalone HTML string.
pub fn render_to_html(text: &str) -> Result<String, RenderError> {
    let parsed = mrml::parse(text).map_err(|e| RenderError::Parse(e.to_string()))?;
    parsed
        .element
        .render(&mrml::prelude::render::RenderOptions::default())
        .map_err(|e| RenderError::Render(e.to_string()))
}

/// Builds a minimal standalone HTML page that displays `message`, so a failed
/// render still opens the browser with something readable instead of nothing.
pub fn error_page_html(message: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>MJML preview error</title>\
         </head><body><pre>{message}</pre></body></html>"
    )
}

/// Returns the argv used to open `path` in the default browser for `os`.
/// Pure: it only builds the argument list and never spawns a process.
pub fn browser_argv(os: TargetOs, path: &str) -> Vec<String> {
    match os {
        TargetOs::Mac => vec!["open".to_string(), path.to_string()],
        TargetOs::Linux => vec!["xdg-open".to_string(), path.to_string()],
        TargetOs::Windows => vec![
            "cmd".to_string(),
            "/C".to_string(),
            "start".to_string(),
            String::new(),
            path.to_string(),
        ],
    }
}

/// Returns the argv to open `path` in the default browser on the current OS.
pub fn browser_command(path: &str) -> Vec<String> {
    browser_argv(current_target_os(), path)
}

/// Spawns the default browser on `path` and returns immediately. It never waits
/// on the child, so a misbehaving opener cannot stall the LSP main loop.
pub fn open_in_browser(path: &Path) -> std::io::Result<()> {
    let path_str = path.to_string_lossy();
    let argv = browser_command(&path_str);
    let Some((program, rest)) = argv.split_first() else {
        return Err(std::io::Error::other("empty browser command"));
    };
    Command::new(program).args(rest).spawn().map(|_| ())
}

/// Writes `html` to `{dir}/{stem}.preview.html` and returns the path. One file
/// per stem (overwritten on each call), so there is nothing to clean up.
pub fn write_html_to(dir: &Path, html: &str, stem: &str) -> std::io::Result<PathBuf> {
    let path = dir.join(format!("{stem}.preview.html"));
    std::fs::write(&path, html)?;
    Ok(path)
}

/// Writes `html` to a `{stem}.preview.html` file in the system temp directory.
pub fn write_temp_html(html: &str, stem: &str) -> std::io::Result<PathBuf> {
    write_html_to(&std::env::temp_dir(), html, stem)
}

/// Maps the compile-time target OS to a [`TargetOs`], defaulting to Linux for
/// anything unrecognized.
const fn current_target_os() -> TargetOs {
    if cfg!(target_os = "macos") {
        TargetOs::Mac
    } else if cfg!(target_os = "linux") {
        TargetOs::Linux
    } else if cfg!(target_os = "windows") {
        TargetOs::Windows
    } else {
        TargetOs::Linux
    }
}

// --- Marker-pulse trigger helpers ------------------------------------------
//
// Zed has no workspace/executeCommand, so a code action cannot ask the server
// to run on selection. Instead, the "Open Preview in Browser" action inserts a
// unique marker comment; the resulting didChange is the only signal that the
// user actually picked it. The server renders, opens the browser, then strips
// the marker via workspace/applyEdit. A handled-set keeps it to one fire per
// selection (the strip's own didChange is a no-op).

const MARKER_PREFIX: &str = "<!--mjml-preview-";
const MARKER_SUFFIX: &str = "-->";

/// Decision made when scanning a document change for a preview marker.
#[derive(Debug, PartialEq, Eq)]
pub enum MarkerDecision {
    /// A fresh marker was applied: fire the preview, then strip bytes [start, end).
    Fire {
        nonce: String,
        start: usize,
        end: usize,
    },
    /// A marker is present but its nonce was already handled (the strip's
    /// didChange, or a stale marker): do nothing.
    Noop,
    /// No marker in the document.
    None,
}

/// The marker comment inserted for a given nonce.
pub fn selection_marker(nonce: u64) -> String {
    format!("{MARKER_PREFIX}{nonce}{MARKER_SUFFIX}")
}

/// Finds a `<!--mjml-preview-<nonce>-->` marker, returning the nonce and the
/// marker's byte span as `(nonce, start, end)`.
pub fn find_preview_marker(text: &str) -> Option<(String, usize, usize)> {
    let start = text.find(MARKER_PREFIX)?;
    let after = &text[start + MARKER_PREFIX.len()..];
    let end_rel = after.find(MARKER_SUFFIX)?;
    let nonce = after[..end_rel].to_string();
    let end = start + MARKER_PREFIX.len() + end_rel + MARKER_SUFFIX.len();
    Some((nonce, start, end))
}

/// Classifies a document change against the set of already-handled nonces.
pub fn classify_marker(text: &str, handled: &HashSet<String>) -> MarkerDecision {
    match find_preview_marker(text) {
        None => MarkerDecision::None,
        Some((nonce, _, _)) if handled.contains(&nonce) => MarkerDecision::Noop,
        Some((nonce, start, end)) => MarkerDecision::Fire { nonce, start, end },
    }
}

/// Builds the no-`command` "Open Preview in Browser" code action that inserts
/// the marker for `nonce` at zero-width position `at`.
pub fn build_preview_action(uri: Uri, nonce: u64, at: Position) -> CodeActionOrCommand {
    let mut changes = HashMap::new();
    changes.insert(
        uri,
        vec![TextEdit {
            range: Range::new(at, at),
            new_text: selection_marker(nonce),
        }],
    );
    CodeActionOrCommand::CodeAction(CodeAction {
        title: "Open Preview in Browser".to_string(),
        kind: Some(CodeActionKind::EMPTY),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        ..Default::default()
    })
}

/// Builds the `workspace/applyEdit` request that removes `range` from `uri`.
pub fn strip_marker_request(id: i32, uri: Uri, range: Range) -> ServerRequest {
    let mut changes = HashMap::new();
    changes.insert(uri, vec![TextEdit { range, new_text: String::new() }]);
    let params = ApplyWorkspaceEditParams {
        label: Some("Remove MJML preview marker".to_string()),
        edit: WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        },
    };
    ServerRequest::new(
        lsp_server::RequestId::from(id),
        "workspace/applyEdit".to_string(),
        params,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = "<mjml><mj-body><mj-section><mj-column><mj-text>Hi</mj-text></mj-column></mj-section></mj-body></mjml>";

    #[test]
    fn renders_minimal_document_to_html() {
        let html = render_to_html(MINIMAL).expect("a minimal valid document should render");
        assert!(html.contains("<html"), "rendered output should be an html document: {html}");
        assert!(html.contains("</html>"));
        assert!(html.contains("Hi"), "rendered output should contain the text content");
    }

    #[test]
    fn returns_parse_error_for_non_mjml_input() {
        let err = render_to_html("not valid mjml at all").unwrap_err();
        assert!(matches!(err, RenderError::Parse(_)), "expected a parse error, got {err:?}");
    }

    #[test]
    fn error_page_contains_message_and_is_html() {
        let page = error_page_html("boom: bad mjml");
        assert!(page.contains("<html"), "error page should be an html document: {page}");
        assert!(page.contains("boom: bad mjml"), "error page should contain the message");
    }

    #[test]
    fn browser_argv_per_os() {
        assert_eq!(
            browser_argv(TargetOs::Mac, "/p.html"),
            vec!["open".to_string(), "/p.html".to_string()]
        );
        assert_eq!(
            browser_argv(TargetOs::Linux, "/p.html"),
            vec!["xdg-open".to_string(), "/p.html".to_string()]
        );
        assert_eq!(
            browser_argv(TargetOs::Windows, "/p.html"),
            vec![
                "cmd".to_string(),
                "/C".to_string(),
                "start".to_string(),
                String::new(),
                "/p.html".to_string(),
            ]
        );
    }

    #[test]
    fn browser_command_matches_current_os() {
        let argv = browser_command("/p.html");
        assert_eq!(argv, browser_argv(current_target_os(), "/p.html"));
        assert!(argv.contains(&"/p.html".to_string()));
    }

    #[test]
    fn write_html_to_creates_file_with_content() {
        let dir = std::env::temp_dir();
        let stem = "mjml_preview_test_unit";
        let path = write_html_to(&dir, "<html>ok</html>", stem).expect("write should succeed");
        assert_eq!(path, dir.join(format!("{stem}.preview.html")));
        let on_disk = std::fs::read_to_string(&path).expect("file should be readable");
        assert_eq!(on_disk, "<html>ok</html>");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn selection_marker_format() {
        assert_eq!(selection_marker(7), "<!--mjml-preview-7-->");
    }

    #[test]
    fn find_marker_locates_nonce_and_span() {
        let text = "abc<!--mjml-preview-42-->def";
        let (nonce, start, end) = find_preview_marker(text).expect("marker should be found");
        assert_eq!(nonce, "42");
        assert_eq!(&text[start..end], "<!--mjml-preview-42-->");
    }

    #[test]
    fn find_marker_returns_none_when_absent() {
        assert!(find_preview_marker("<mjml></mjml>").is_none());
    }

    #[test]
    fn classify_marker_fires_for_new_nonce() {
        let handled = HashSet::new();
        let text = format!("x{}y", selection_marker(1));
        match classify_marker(&text, &handled) {
            MarkerDecision::Fire { nonce, .. } => assert_eq!(nonce, "1"),
            other => panic!("expected Fire, got {other:?}"),
        }
    }

    #[test]
    fn classify_marker_noop_for_handled_nonce() {
        let mut handled = HashSet::new();
        handled.insert("1".to_string());
        let text = format!("x{}y", selection_marker(1));
        assert_eq!(classify_marker(&text, &handled), MarkerDecision::Noop);
    }

    #[test]
    fn classify_marker_none_when_no_marker() {
        let handled = HashSet::new();
        assert_eq!(classify_marker("plain text", &handled), MarkerDecision::None);
    }

    #[test]
    fn build_preview_action_has_no_command_and_inserts_marker() {
        let uri: Uri = "file:///tmp/x.mjml".parse().unwrap();
        let at = Position::new(0, 0);
        let action = build_preview_action(uri.clone(), 5, at);
        let CodeActionOrCommand::CodeAction(ca) = action else {
            panic!("expected a CodeAction, got a Command");
        };
        assert!(ca.command.is_none(), "action must not carry a command (Zed ignores it)");
        let changes = ca.edit.and_then(|e| e.changes).expect("edit should have changes");
        let edits = changes.get(&uri).expect("edit should target the uri");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, selection_marker(5));
        assert_eq!(edits[0].range, Range::new(at, at));
    }

    #[test]
    fn strip_marker_request_targets_apply_edit_with_empty_text() {
        let uri: Uri = "file:///tmp/x.mjml".parse().unwrap();
        let range = Range::new(Position::new(1, 0), Position::new(1, 10));
        let req = strip_marker_request(99, uri, range.clone());
        assert_eq!(req.method, "workspace/applyEdit");
        let params: ApplyWorkspaceEditParams =
            serde_json::from_value(req.params).expect("params should deserialize");
        let changes = params.edit.changes.expect("changes should be present");
        let edits = changes.values().next().expect("at least one edit");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "");
        assert_eq!(edits[0].range, range);
    }
}
