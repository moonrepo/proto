use super::process::{ProtoProcessError, exec_command_piped, handle_exec};
use proto_pdk_api::ArchiveSource;
use starbase_archive::{ArchiveError, Archiver};
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::net::{DownloadOptions, NetError};
use starbase_utils::{fs, net};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::process::Command;
use warpgate::extract_file_name_from_url;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoArchiveError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Archive(#[from] Box<ArchiveError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Net(#[from] Box<NetError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Process(#[from] Box<ProtoProcessError>),

    #[diagnostic(code(proto::archive::missing_pkg_payload))]
    #[error("Unable to find a payload in macOS package {}.", .path.style(Style::Path))]
    MissingPkgPayload { path: PathBuf },

    #[diagnostic(code(proto::archive::missing_pkg_contents))]
    #[error(
        "Unable to extract contents from macOS package {}, using directory prefix {}.",
        .path.style(Style::Path),
        .prefix.style(Style::Label)
    )]
    MissingPkgContents { path: PathBuf, prefix: String },
}

impl From<ArchiveError> for ProtoArchiveError {
    fn from(e: ArchiveError) -> ProtoArchiveError {
        ProtoArchiveError::Archive(Box::new(e))
    }
}

impl From<FsError> for ProtoArchiveError {
    fn from(e: FsError) -> ProtoArchiveError {
        ProtoArchiveError::Fs(Box::new(e))
    }
}

impl From<NetError> for ProtoArchiveError {
    fn from(e: NetError) -> ProtoArchiveError {
        ProtoArchiveError::Net(Box::new(e))
    }
}

impl From<ProtoProcessError> for ProtoArchiveError {
    fn from(error: ProtoProcessError) -> ProtoArchiveError {
        ProtoArchiveError::Process(Box::new(error))
    }
}

pub fn should_unpack(src: &ArchiveSource, target_dir: &Path) -> Result<bool, ProtoArchiveError> {
    let url_file = target_dir.join(".archive-url");
    let mut unpack = true;

    // If the URLs have changed at some point, we need to remove
    // the current files, and download new ones
    if url_file.exists() {
        let previous_url = fs::read_file(&url_file)?;

        if src.url.trim() == previous_url.trim() {
            unpack = false;
        } else {
            fs::remove_dir_all(target_dir)?;
        }
    }

    fs::create_dir_all(target_dir)?;

    Ok(unpack)
}

pub async fn download(
    src: &ArchiveSource,
    temp_dir: &Path,
    options: DownloadOptions,
) -> Result<PathBuf, ProtoArchiveError> {
    let filename = extract_file_name_from_url(&src.url);
    let archive_file = temp_dir.join(&filename);

    net::download_from_url_with_options(&src.url, &archive_file, options).await?;

    Ok(archive_file)
}

pub async fn download_and_unpack(
    src: &ArchiveSource,
    target_dir: &Path,
    temp_dir: &Path,
    options: DownloadOptions,
) -> Result<(), ProtoArchiveError> {
    if should_unpack(src, target_dir)? {
        let archive_file = download(src, temp_dir, options).await?;

        unpack_source(src, target_dir, temp_dir, &archive_file).await?;
    }

    Ok(())
}

pub async fn unpack_source(
    src: &ArchiveSource,
    target_dir: &Path,
    temp_dir: &Path,
    archive_file: &Path,
) -> Result<(String, PathBuf), ProtoArchiveError> {
    let result = unpack(target_dir, temp_dir, archive_file, src.prefix.as_deref()).await;

    fs::write_file(target_dir.join(".archive-url"), &src.url)?;

    result
}

pub async fn unpack(
    target_dir: &Path,
    temp_dir: &Path,
    archive_file: &Path,
    prefix: Option<&str>,
) -> Result<(String, PathBuf), ProtoArchiveError> {
    match archive_file.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("pkg") => {
            unpack_pkg(target_dir, temp_dir, archive_file, prefix).await?;

            Ok(("pkg".into(), target_dir.to_path_buf()))
        }
        _ => {
            let mut archiver = Archiver::new(target_dir, archive_file);

            if let Some(prefix) = prefix {
                archiver.set_prefix(prefix);
            }

            Ok(archiver.unpack_from_ext()?)
        }
    }
}

async fn unpack_pkg(
    target_dir: &Path,
    temp_dir: &Path,
    archive_file: &Path,
    prefix: Option<&str>,
) -> Result<(), ProtoArchiveError> {
    let expanded_dir = temp_dir.join("pkg");
    let payload_dir = expanded_dir.join("Payload");

    // Remove expanded dir if it exists
    fs::remove_dir_all(&expanded_dir)?;

    handle_exec(
        exec_command_piped(
            Command::new("pkgutil")
                .arg("--expand-full")
                .arg(archive_file)
                .arg(&expanded_dir),
        )
        .await?,
    )?;

    if !payload_dir.exists() {
        return Err(ProtoArchiveError::MissingPkgPayload {
            path: expanded_dir.to_path_buf(),
        });
    }

    copy_extracted_contents(&payload_dir, target_dir, prefix)?;

    Ok(())
}

fn copy_extracted_contents(
    source_dir: &Path,
    target_dir: &Path,
    prefix: Option<&str>,
) -> Result<(), ProtoArchiveError> {
    let source = match prefix {
        Some(prefix) => source_dir.join(prefix),
        None => source_dir.to_path_buf(),
    };

    if !source.exists() {
        return Err(ProtoArchiveError::MissingPkgContents {
            path: source_dir.to_path_buf(),
            prefix: prefix.unwrap_or("N/A").into(),
        });
    } else if source.is_file() {
        fs::copy_file(&source, target_dir.join(fs::file_name(&source)))?;
    } else {
        fs::copy_dir_all(&source, target_dir)?;
    }

    Ok(())
}
