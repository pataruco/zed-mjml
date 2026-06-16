use std::fs;
use zed_extension_api::{self as zed, DownloadedFileType, GithubReleaseOptions, Result};

struct MjmlExtension {
    cached_binary_path: Option<String>,
}

impl MjmlExtension {
    fn language_server_binary_path(
        &mut self,
        language_server_id: &zed::LanguageServerId,
    ) -> Result<String> {
        // If we have a cached path and the file still exists, return it.
        if let Some(ref path) = self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|m| m.is_file()) {
                return Ok(path.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            "pataruco/zed-mjml",
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )
        .map_err(|e| format!("failed to fetch latest release: {e}"))?;

        let (os, arch) = zed::current_platform();

        let os_str = match os {
            zed::Os::Mac => "apple-darwin",
            zed::Os::Linux => "unknown-linux-gnu",
            zed::Os::Windows => "pc-windows-msvc",
        };

        let arch_str = match arch {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X8664 => "x86_64",
            zed::Architecture::X86 => "x86",
        };

        let asset_name = format!("mjml-lsp-{arch_str}-{os_str}.gz");

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| {
                format!(
                    "no matching binary for platform {arch_str}-{os_str} in release {}",
                    release.version
                )
            })?;

        let version_dir = format!("mjml-lsp-{}", release.version);
        let binary_path = format!("{version_dir}/mjml-lsp");

        // If the binary for this version already exists on disk, use it.
        if fs::metadata(&binary_path).is_ok_and(|m| m.is_file()) {
            self.cached_binary_path = Some(binary_path.clone());
            return Ok(binary_path);
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        // `download_file` does not create parent directories for a `Gzip` (single-file)
        // download, so the version directory must exist beforehand.
        fs::create_dir_all(&version_dir)
            .map_err(|e| format!("failed to create directory `{version_dir}`: {e}"))?;

        zed::download_file(&asset.download_url, &binary_path, DownloadedFileType::Gzip)
            .map_err(|e| format!("failed to download file: {e}"))?;

        zed::make_file_executable(&binary_path)
            .map_err(|e| format!("failed to make file executable: {e}"))?;

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
}

zed::register_extension!(MjmlExtension);
