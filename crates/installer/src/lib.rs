mod error;

use futures::StreamExt;
use starbase_archive::Archiver;
use starbase_utils::fs::{self, FsError};
use std::cmp;
use std::env;
use std::env::consts;
use std::fmt::Debug;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use system_env::SystemLibc;
use tracing::instrument;

pub use error::ProtoInstallerError;

#[instrument]
pub fn determine_triple() -> miette::Result<String> {
    let target = match (consts::OS, consts::ARCH) {
        ("linux", arch) => format!(
            "{arch}-unknown-linux-{}",
            if SystemLibc::is_musl() { "musl" } else { "gnu" }
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

#[derive(Debug)]
pub struct DownloadResult {
    pub archive_file: PathBuf,
    pub file: String,
    pub file_stem: String,
    pub url: String,
}

#[instrument(skip(on_chunk))]
pub async fn download_release(
    triple: &str,
    version: &str,
    temp_dir: impl AsRef<Path> + Debug,
    on_chunk: impl Fn(u64, u64),
) -> miette::Result<DownloadResult> {
    let target_ext = if cfg!(windows) { "zip" } else { "tar.xz" };
    let target_file = format!("proto_cli-{triple}");

    let download_file = format!("{target_file}.{target_ext}");
    let download_url =
        format!("https://github.com/moonrepo/proto/releases/download/v{version}/{download_file}");

    // Request file from url
    let handle_error = |error: reqwest::Error| ProtoInstallerError::DownloadFailed {
        url: download_url.clone(),
        error: Box::new(error),
    };
    let response = reqwest::Client::new()
        .get(&download_url)
        .send()
        .await
        .map_err(handle_error)?;
    let total_size = response.content_length().unwrap_or(0);

    on_chunk(0, total_size);

    // Download in chunks
    let archive_file = temp_dir.as_ref().join(&download_file);
    let mut file = fs::create_file(&archive_file)?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(handle_error)?;

        file.write_all(&chunk).map_err(|error| FsError::Write {
            path: archive_file.to_path_buf(),
            error: Box::new(error),
        })?;

        downloaded = cmp::min(downloaded + (chunk.len() as u64), total_size);

        on_chunk(downloaded, total_size);
    }

    Ok(DownloadResult {
        archive_file,
        file: download_file,
        file_stem: target_file,
        url: download_url,
    })
}

#[instrument]
pub fn unpack_release(
    download: DownloadResult,
    install_dir: impl AsRef<Path> + Debug,
    relocate_dir: impl AsRef<Path> + Debug,
    relocate_current: bool,
) -> miette::Result<bool> {
    let temp_dir = download
        .archive_file
        .parent()
        .unwrap()
        .join(&download.file_stem);
    let install_dir = install_dir.as_ref();
    let bin_names = if cfg!(windows) {
        vec!["proto.exe", "proto-shim.exe"]
    } else {
        vec!["proto", "proto-shim"]
    };

    // Unpack the downloaded file
    Archiver::new(&temp_dir, &download.archive_file).unpack_from_ext()?;

    // Move the old binaries
    let relocate = |current_path: &Path, relocate_path: &Path| -> miette::Result<()> {
        fs::rename(current_path, relocate_path)?;

        // Track last used so operations like clean continue to work
        // correctly, otherwise we get into a weird state!
        fs::write_file(
            relocate_path.parent().unwrap().join(".last-used"),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
                .to_string(),
        )?;

        Ok(())
    };

    for bin_name in &bin_names {
        let output_path = install_dir.join(bin_name);
        let relocate_path = relocate_dir.as_ref().join(bin_name);

        if output_path.exists() && output_path != relocate_path {
            relocate(&output_path, &relocate_path)?;
        }

        // If not installed at our standard location
        if relocate_current {
            if let Ok(current_exe) = env::current_exe() {
                if current_exe != output_path
                    && current_exe
                        .file_name()
                        .is_some_and(|name| name == *bin_name)
                {
                    relocate(&current_exe, &relocate_path)?;
                }
            }
        }
    }

    // Move the new binary to the bins directory
    let mut unpacked = false;

    for bin_name in &bin_names {
        let output_path = install_dir.join(bin_name);
        let input_paths = vec![
            temp_dir.join(&download.file_stem).join(bin_name),
            temp_dir.join(bin_name),
        ];

        for input_path in input_paths {
            if input_path.exists() {
                fs::copy_file(input_path, &output_path)?;
                fs::update_perms(&output_path, None)?;

                unpacked = true;
                break;
            }
        }
    }

    fs::remove(temp_dir)?;
    fs::remove(download.archive_file)?;

    Ok(unpacked)
}
