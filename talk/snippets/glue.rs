let release = zed::latest_github_release(
    "pataruco/zed-mjml",
    GithubReleaseOptions { require_assets: true, pre_release: false },
)?;

let (os, arch) = zed::current_platform();
let asset_name = format!("mjml-lsp-{arch_str}-{os_str}.gz");

zed::download_file(&asset.download_url, &binary_path, DownloadedFileType::Gzip)?;
zed::make_file_executable(&binary_path)?;

Ok(zed::Command {
    command: binary_path,
    args: vec!["--stdio".to_string()],
    env: Vec::default(),
})
