use crate::client::HttpClient;
use crate::loader_error::WarpgateLoaderError;
use sha2::{Digest, Sha256};
use starbase_archive::{Archiver, is_supported_archive_extension};
use starbase_utils::{fs, glob, net, net::NetError};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tracing::instrument;
use warpgate_api::VirtualPath;

/// Create a SHA256 hash key based on the provided URL and seed.
pub fn create_cache_key(url: &str, seed: Option<&str>) -> String {
    let mut sha = Sha256::new();
    sha.update(url);

    if let Some(seed) = seed {
        sha.update(seed);
    }

    format!("{:x}", sha.finalize())
}

/// Determine the extension to use for a cache file, based on our
/// list of supported extensions.
pub fn determine_cache_extension(value: &str) -> Option<&str> {
    [".toml", ".json", ".jsonc", ".yaml", ".yml", ".wasm", ".txt"]
        .into_iter()
        .find(|ext| value.ends_with(ext))
}

/// Download a file from the provided URL, with the provided HTTP(S)
/// client, and save it to a destination location.
#[instrument(skip(client))]
pub async fn download_from_url_to_file(
    source_url: &str,
    dest_file: &Path,
    client: &HttpClient,
) -> Result<(), WarpgateLoaderError> {
    if let Err(error) = net::download_from_url_with_options(
        source_url,
        dest_file,
        net::DownloadOptions {
            downloader: Some(Box::new(client.create_downloader())),
            ..Default::default()
        },
    )
    .await
    {
        return Err(match error {
            NetError::UrlNotFound { url } => WarpgateLoaderError::NotFound { url }.into(),
            e => WarpgateLoaderError::FailedDownload {
                url: source_url.into(),
                error: Box::new(e),
            },
        });
    };

    Ok(())
}

/// If the temporary file is an archive, unpack it into the destination,
/// otherwise more the file into the destination.
#[instrument]
pub fn move_or_unpack_download(
    temp_file: &Path,
    dest_file: &Path,
) -> Result<(), WarpgateLoaderError> {
    // Archive supported file extensions
    if is_supported_archive_extension(temp_file) {
        let out_dir = temp_file.parent().unwrap().join("out");

        Archiver::new(&out_dir, temp_file).unpack_from_ext()?;

        let wasm_files = glob::walk_files(&out_dir, ["**/*.wasm"])?;

        if wasm_files.is_empty() {
            return Err(WarpgateLoaderError::NoWasmFound {
                path: temp_file.to_path_buf(),
            }
            .into());

            // Find a release file first, as some archives include the target folder
        } else if let Some(release_wasm) = wasm_files
            .iter()
            .find(|file| file.to_string_lossy().contains("release"))
        {
            fs::rename(release_wasm, dest_file)?;

            // Otherwise, move the first wasm file available
        } else {
            fs::rename(&wasm_files[0], dest_file)?;
        }

        fs::remove_file(temp_file)?;
        fs::remove_dir_all(out_dir)?;
    }

    // Non-archive supported extensions
    match temp_file.extension().and_then(|ext| ext.to_str()) {
        Some("wasm" | "toml" | "json" | "jsonc" | "yaml" | "yml") => {
            fs::rename(temp_file, dest_file)?;
        }

        Some(ext) => {
            return Err(WarpgateLoaderError::UnsupportedDownloadExtension {
                ext: ext.to_owned(),
                path: temp_file.to_path_buf(),
            }
            .into());
        }

        None => {
            return Err(WarpgateLoaderError::UnknownDownloadType {
                path: temp_file.to_path_buf(),
            }
            .into());
        }
    };

    Ok(())
}

/// Sort virtual paths from longest to shortest host path,
/// so that prefix replacing is deterministic and accurate.
fn sort_virtual_paths(map: &BTreeMap<PathBuf, PathBuf>) -> Vec<(&PathBuf, &PathBuf)> {
    let mut list = map.iter().collect::<Vec<_>>();
    list.sort_by(|a, d| d.0.cmp(a.0));
    list
}

/// Convert the provided virtual guest path to an absolute host path.
#[instrument]
pub fn from_virtual_path(
    paths_map: &BTreeMap<PathBuf, PathBuf>,
    path: impl AsRef<Path> + Debug,
) -> PathBuf {
    let path = path.as_ref();

    for (host_path, guest_path) in sort_virtual_paths(paths_map) {
        if let Ok(rel_path) = path.strip_prefix(guest_path) {
            let real_path = host_path.join(rel_path);

            return prepare_from_path(&real_path);
        }
    }

    prepare_from_path(path)
}

/// Convert the provided absolute host path to a virtual guest path suitable
/// for WASI sandboxed runtimes.
#[instrument]
pub fn to_virtual_path(
    paths_map: &BTreeMap<PathBuf, PathBuf>,
    path: impl AsRef<Path> + Debug,
) -> VirtualPath {
    let path = path.as_ref();

    for (host_path, guest_path) in sort_virtual_paths(paths_map) {
        let virtual_path = if path.starts_with(guest_path) {
            path.to_owned()
        } else if let Ok(rel_path) = path.strip_prefix(host_path) {
            guest_path.join(rel_path)
        } else {
            continue;
        };

        return VirtualPath::Virtual {
            path: prepare_to_path(&virtual_path),
            virtual_prefix: prepare_to_path(guest_path),
            real_prefix: prepare_to_path(host_path),
        };
    }

    VirtualPath::Real(prepare_to_path(path))
}

#[cfg(unix)]
fn prepare_to_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

#[cfg(unix)]
fn prepare_from_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

// Only forward slashes are allowed in WASI. This is also required
// when joining paths in WASM, because mismatched separators will
// cause issues.

#[cfg(windows)]
fn prepare_to_path(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('\\', "/"))
}

#[cfg(windows)]
fn prepare_from_path(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('/', "\\"))
}
