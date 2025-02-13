use crate::helpers::extract_filename_from_url;
use starbase_archive::Archiver;
use starbase_utils::net;
use std::path::{Path, PathBuf};

pub async fn download(
    url: &str,
    temp_dir: &Path,
    client: &reqwest::Client,
) -> miette::Result<PathBuf> {
    let filename = extract_filename_from_url(url)?;
    let archive_file = temp_dir.join(&filename);

    net::download_from_url_with_client(url, &archive_file, client).await?;

    Ok(archive_file)
}

pub fn unpack(
    target_dir: &Path,
    archive_file: &Path,
    prefix: Option<&str>,
) -> miette::Result<(String, PathBuf)> {
    let mut archiver = Archiver::new(target_dir, archive_file);

    if let Some(prefix) = prefix {
        archiver.set_prefix(prefix);
    }

    archiver.unpack_from_ext()
}
