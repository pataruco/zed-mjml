// TODO: Temporary local binary for testing. Revert before publishing.
use zed_extension_api::{self as zed, Result};

struct MjmlExtension;

impl zed::Extension for MjmlExtension {
    fn new() -> Self {
        MjmlExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Ok(zed::Command {
            command: "/Users/pataruco/dev/zed-mjml/crates/mjml-lsp/target/debug/mjml-lsp"
                .to_string(),
            args: vec!["--stdio".to_string()],
            env: Default::default(),
        })
    }
}

zed::register_extension!(MjmlExtension);
