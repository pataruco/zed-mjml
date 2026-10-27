fn language_server_command(
    &mut self,
    language_server_id: &zed::LanguageServerId,
    _worktree: &zed::Worktree,
) -> Result<zed::Command> {
    let binary_path = self.language_server_binary_path(language_server_id)?;
    Ok(zed::Command {
        command: binary_path,
        args: vec!["--stdio".to_string()],
        env: Vec::default(),
    })
}
