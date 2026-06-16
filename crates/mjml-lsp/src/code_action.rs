use std::collections::HashMap;

use lsp_server::{Request, Response};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    Diagnostic, Range, TextEdit, Uri, WorkspaceEdit,
};
use serde::{Deserialize, Serialize};

/// Serializable fix payload attached to a diagnostic's `data` field.
///
/// Diagnostics carry their own fix so the code-action handler can build a
/// `WorkspaceEdit` without re-analyzing the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticFix {
    pub title: String,
    pub edits: Vec<FixEdit>,
}

/// A single text replacement within a fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixEdit {
    pub range: Range,
    pub new_text: String,
}

/// Handles a `textDocument/codeAction` request and returns the available actions.
pub fn handle(req: &Request) -> Response {
    let actions = code_actions(req).unwrap_or_default();
    Response::new_ok(req.id.clone(), actions)
}

fn code_actions(req: &Request) -> Option<CodeActionResponse> {
    let params = serde_json::from_value::<CodeActionParams>(req.params.clone()).ok()?;
    Some(build_actions(
        &params.text_document.uri,
        &params.context.diagnostics,
    ))
}

/// Builds quick-fix code actions from the fixes embedded in `diagnostics`.
fn build_actions(uri: &Uri, diagnostics: &[Diagnostic]) -> CodeActionResponse {
    diagnostics
        .iter()
        .filter_map(|diag| {
            let fix: DiagnosticFix = serde_json::from_value(diag.data.clone()?).ok()?;
            let edits: Vec<TextEdit> = fix
                .edits
                .into_iter()
                .map(|e| TextEdit {
                    range: e.range,
                    new_text: e.new_text,
                })
                .collect();
            let mut changes = HashMap::new();
            changes.insert(uri.clone(), edits);
            Some(CodeActionOrCommand::CodeAction(CodeAction {
                title: fix.title,
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                }),
                is_preferred: Some(true),
                ..Default::default()
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{DiagnosticSeverity, Position};

    fn uri() -> Uri {
        "file:///test.mjml".parse().unwrap()
    }

    fn diagnostic(data: Option<serde_json::Value>) -> Diagnostic {
        Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
            severity: Some(DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("mjml".to_string()),
            message: "msg".to_string(),
            related_information: None,
            tags: None,
            data,
        }
    }

    #[test]
    fn builds_quick_fix_from_diagnostic_data() {
        let fix = DiagnosticFix {
            title: "Replace with <mj-section>".to_string(),
            edits: vec![FixEdit {
                range: Range::new(Position::new(0, 1), Position::new(0, 11)),
                new_text: "mj-section".to_string(),
            }],
        };
        let data = serde_json::to_value(fix).unwrap();
        let actions = build_actions(&uri(), &[diagnostic(Some(data))]);

        assert_eq!(actions.len(), 1);
        let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
            panic!("expected a code action");
        };
        assert_eq!(action.title, "Replace with <mj-section>");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));

        let changes = action.edit.as_ref().unwrap().changes.as_ref().unwrap();
        let edits = changes.get(&uri()).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "mj-section");
    }

    #[test]
    fn ignores_diagnostics_without_fix_data() {
        assert!(build_actions(&uri(), &[diagnostic(None)]).is_empty());
    }
}
