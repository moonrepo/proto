mod error;

use futures::StreamExt;
use starbase_archive::Archiver;
use starbase_utils::fs::{self, FsError};
use std::cmp;
use std::env;
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

pub struct DownloadResult {
    pub archive_file: PathBuf,
    pub file: String,
    pub file_stem: String,
    pub url: String,
    pub version: String,
}

pub async fn download_release(
    triple: &str,
    version: &str,
    temp_dir: &Path,
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
    let archive_file = temp_dir.join(&download_file);
    let mut file = fs::create_file(&archive_file)?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(handle_error)?;

        file.write_all(&chunk).map_err(|error| FsError::Write {
            path: archive_file.to_path_buf(),
            error,
        })?;

        downloaded = cmp::min(downloaded + (chunk.len() as u64), total_size);

        on_chunk(downloaded, total_size);
    }

    Ok(DownloadResult {
        archive_file,
        file: download_file,
        file_stem: target_file,
        url: download_url,
        version: version.to_owned(),
    })
}

pub fn unpack_release(
    download: DownloadResult,
    install_dir: &Path,
    tools_dir: &Path,
) -> miette::Result<bool> {
    let temp_dir = download
        .archive_file
        .parent()
        .unwrap()
        .join(&download.file_stem);

    // Unpack the downloaded file
    Archiver::new(&temp_dir, &download.archive_file).unpack_from_ext()?;

    // Move the old binaries
    let bin_names = if cfg!(windows) {
        vec!["proto.exe", "proto-shim.exe"]
    } else {
        vec!["proto", "proto-shim"]
    };
    let bin_dir = match env::var("PROTO_INSTALL_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => install_dir.to_owned(),
    };

    for bin_name in &bin_names {
        let output_path = bin_dir.join(bin_name);
        let relocate_path = tools_dir
            .join("proto")
            .join(&download.version)
            .join(bin_name);

        if output_path.exists() {
            fs::rename(&output_path, &relocate_path)?;
        }

        // If not installed at our standard location
        if let Ok(current_exe) = env::current_exe() {
            if current_exe != output_path
                && current_exe
                    .file_name()
                    .is_some_and(|name| name == *bin_name)
            {
                fs::rename(&current_exe, &relocate_path)?;
            }
        }
    }

    // Move the new binary to the bins directory
    let mut unpacked = false;

    for bin_name in &bin_names {
        let output_path = bin_dir.join(bin_name);
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
