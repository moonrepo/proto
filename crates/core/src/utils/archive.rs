use crate::helpers::extract_filename_from_url;
use proto_pdk_api::ArchiveSource;
use starbase_archive::Archiver;
use starbase_utils::fs::FsError;
use starbase_utils::net::NetError;
use starbase_utils::{fs, net};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoArchiveError {
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[error(transparent)]
    Net(#[from] Box<NetError>),
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
    client: &reqwest::Client,
) -> Result<PathBuf, ProtoArchiveError> {
    let filename = extract_filename_from_url(&src.url)?;
    let archive_file = temp_dir.join(&filename);

    net::download_from_url_with_client(&src.url, &archive_file, client).await?;

    Ok(archive_file)
}

pub fn unpack(
    src: &ArchiveSource,
    target_dir: &Path,
    archive_file: &Path,
) -> Result<(String, PathBuf), ProtoArchiveError> {
    let result = unpack_raw(target_dir, archive_file, src.prefix.as_deref());

    fs::write_file(target_dir.join(".archive-url"), &src.url)?;

    result
}

pub fn unpack_raw(
    target_dir: &Path,
    archive_file: &Path,
    prefix: Option<&str>,
) -> Result<(String, PathBuf), ProtoArchiveError> {
    let mut archiver = Archiver::new(target_dir, archive_file);

    if let Some(prefix) = prefix {
        archiver.set_prefix(prefix);
    }

    archiver.unpack_from_ext()
}

pub async fn download_and_unpack(
    src: &ArchiveSource,
    target_dir: &Path,
    temp_dir: &Path,
    client: &reqwest::Client,
) -> Result<(), ProtoArchiveError> {
    if should_unpack(src, target_dir)? {
        let archive_file = download(src, temp_dir, client).await?;

        unpack(src, target_dir, &archive_file)?;
    }

    Ok(())
}
