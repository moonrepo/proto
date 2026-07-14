use super::process::ProtoProcessError;
use proto_pdk_api::ArchiveSource;
use rustc_hash::FxHashMap;
use starbase_archive::{ArchiveError, Archiver};
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::glob::{GlobError, GlobWalkOptions};
use starbase_utils::net::{DownloadOptions, NetError};
use starbase_utils::{fs, glob, net};
use std::path::{Path, PathBuf};
use thiserror::Error;
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
    Glob(#[from] Box<GlobError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Net(#[from] Box<NetError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Process(#[from] Box<ProtoProcessError>),

    #[diagnostic(code(proto::archive::invalid_rewrite))]
    #[error(
        "Invalid archive prefix rewrite {}. Paths must be relative and cannot traverse outside the archive.",
        .path.style(Style::File),
    )]
    InvalidRewritePath { path: String },
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

impl From<GlobError> for ProtoArchiveError {
    fn from(e: GlobError) -> ProtoArchiveError {
        ProtoArchiveError::Glob(Box::new(e))
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

pub fn should_unpack(source: &ArchiveSource, target_dir: &Path) -> Result<bool, ProtoArchiveError> {
    let url_file = target_dir.join(".archive-url");
    let mut unpack = true;

    // If the URLs have changed at some point, we need to remove
    // the current files, and download new ones
    if url_file.exists() {
        let previous_url = fs::read_file(&url_file)?;

        if source.url.trim() == previous_url.trim() {
            unpack = false;
        } else {
            fs::remove_dir_all(target_dir)?;
        }
    }

    fs::create_dir_all(target_dir)?;

    Ok(unpack)
}

pub async fn download(
    source: &ArchiveSource,
    temp_dir: &Path,
    options: DownloadOptions,
) -> Result<PathBuf, ProtoArchiveError> {
    let filename = extract_file_name_from_url(&source.url);
    let archive_file = temp_dir.join(&filename);

    net::download_from_url_with_options(&source.url, &archive_file, options).await?;

    Ok(archive_file)
}

pub async fn download_and_unpack(
    source: &ArchiveSource,
    target_dir: &Path,
    temp_dir: &Path,
    options: DownloadOptions,
) -> Result<(), ProtoArchiveError> {
    if should_unpack(source, target_dir)? {
        let archive_file = download(source, temp_dir, options).await?;

        unpack_source(source, target_dir, &archive_file).await?;
    }

    Ok(())
}

pub async fn unpack_source(
    source: &ArchiveSource,
    target_dir: &Path,
    archive_file: &Path,
) -> Result<(String, PathBuf), ProtoArchiveError> {
    let result = unpack(target_dir, archive_file, source.prefix.as_deref()).await;

    fs::write_file(target_dir.join(".archive-url"), &source.url)?;

    result
}

pub async fn unpack(
    target_dir: &Path,
    archive_file: &Path,
    prefix: Option<&str>,
) -> Result<(String, PathBuf), ProtoArchiveError> {
    let mut archiver = Archiver::new(target_dir, archive_file);

    if let Some(prefix) = prefix {
        archiver.set_prefix(prefix);
    }

    Ok(archiver.unpack_from_ext()?)
}

pub fn move_and_rewrite(
    source_dir: &Path,
    target_dir: &Path,
    rewrites: &FxHashMap<String, String>,
) -> Result<(), ProtoArchiveError> {
    fn is_invalid(value: &str) -> bool {
        value.starts_with("/")
            || value.starts_with("\\")
            || value.contains("../")
            || value == ".."
            || Path::new(value).is_absolute()
    }

    // Rewrite all directories from the source directory first
    for (from_glob, to_prefix) in rewrites {
        if is_invalid(&from_glob) || is_invalid(&to_prefix) {
            return Err(ProtoArchiveError::InvalidRewritePath {
                path: format!("{} -> {}", from_glob, to_prefix),
            });
        }

        for dir in glob::walk_fast_with_options(
            source_dir,
            [from_glob],
            GlobWalkOptions {
                only_dirs: true,
                ..Default::default()
            },
        )? {
            fs::rename(dir, target_dir.join(to_prefix))?;
        }
    }

    // Then move all remaining files last
    for entry in fs::read_dir(source_dir)? {
        let path = entry.path();

        fs::rename(
            &path,
            target_dir.join(path.strip_prefix(source_dir).unwrap()),
        )?;
    }

    Ok(())
}
