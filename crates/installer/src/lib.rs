mod error;

use futures::StreamExt;
use starbase_utils::fs::{self, FsError};
use std::cmp;
use std::env::consts;
use std::io::Write;
use std::path::{Path, PathBuf};

pub use error::ProtoInstallerError;

pub fn is_musl() -> bool {
    let Ok(output) = std::process::Command::new("ldd").arg("--version").output() else {
        return false;
    };

    String::from_utf8(output.stdout).map_or(false, |out| out.contains("musl"))
}

pub fn determine_triple() -> miette::Result<String> {
    let target = match (consts::OS, consts::ARCH) {
        ("linux", arch) => format!(
            "{arch}-unknown-linux-{}",
            if is_musl() { "musl" } else { "gnu" }
        ),
        ("macos", arch) => format!("{arch}-apple-darwin"),
        ("windows", "x86_64") => "x86_64-pc-windows-msvc".to_owned(),
        (os, arch) => {
            return Err(ProtoInstallerError::InvalidPlatform {
                arch: arch.to_owned(),
                os: os.to_owned(),
            }
            .into());
        }
    };

    Ok(target)
}

pub async fn download_release(
    triple: &str,
    version: &str,
    dest_dir: &Path,
    on_chunk: impl Fn(u64, u64),
) -> miette::Result<PathBuf> {
    let target_ext = if cfg!(windows) { "zip" } else { "tar.xz" };
    let target_file = format!("proto_cli-{triple}");

    let download_file = format!("{target_file}.{target_ext}");
    let download_url =
        format!("https://github.com/moonrepo/proto/releases/download/v{version}/{download_file}");

    // Request file from url
    let handle_error = |error: reqwest::Error| ProtoInstallerError::DownloadFailed {
        url: download_url.clone(),
        error,
    };
    let response = reqwest::Client::new()
        .get(&download_url)
        .send()
        .await
        .map_err(handle_error)?;
    let total_size = response.content_length().unwrap_or(0);

    on_chunk(0, total_size);

    // Download in chunks
    let dest_file = dest_dir.join(download_file);
    let mut file = fs::create_file(&dest_file)?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(handle_error)?;

        file.write_all(&chunk).map_err(|error| FsError::Write {
            path: dest_file.to_path_buf(),
            error,
        })?;

        downloaded = cmp::min(downloaded + (chunk.len() as u64), total_size);

        on_chunk(downloaded, total_size);
    }

    Ok(dest_file)
}
