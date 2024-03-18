use crate::error::WarpgateError;
use starbase_archive::{get_full_file_extension, Archiver};
use starbase_utils::{fs, glob, net, net::NetError};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use warpgate_api::VirtualPath;

pub fn determine_cache_extension(value: &str) -> &str {
    for ext in [".toml", ".json", ".jsonc", ".yaml", ".yml"] {
        if value.ends_with(ext) {
            return ext;
        }
    }

    ".wasm"
}

pub async fn download_from_url_to_file(
    source_url: &str,
    temp_file: &Path,
    client: &reqwest::Client,
) -> miette::Result<()> {
    if let Err(error) = net::download_from_url_with_client(source_url, temp_file, client).await {
        return Err(match error {
            NetError::UrlNotFound { url } => WarpgateError::DownloadNotFound { url }.into(),
            _ => error.into(),
        });
    };

    Ok(())
}

pub fn move_or_unpack_download(temp_file: &Path, dest_file: &Path) -> miette::Result<()> {
    // Archive supported file extensions
    if get_full_file_extension(temp_file).is_some() {
        let out_dir = temp_file.parent().unwrap().join("out");

        Archiver::new(&out_dir, temp_file).unpack_from_ext()?;

        let wasm_files = glob::walk_files(&out_dir, ["**/*.wasm"])?;

        if wasm_files.is_empty() {
            return Err(WarpgateError::DownloadNoWasm {
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
            return Err(WarpgateError::DownloadUnsupportedExtension {
                ext: ext.to_owned(),
                path: temp_file.to_path_buf(),
            }
            .into());
        }

        None => {
            return Err(WarpgateError::DownloadUnknownType {
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
pub fn from_virtual_path(
    paths_map: &BTreeMap<PathBuf, PathBuf>,
    path: impl AsRef<Path>,
) -> PathBuf {
    let path = path.as_ref();

    for (host_path, guest_path) in sort_virtual_paths(paths_map) {
        if let Ok(rel_path) = path.strip_prefix(guest_path) {
            let real_path = host_path.join(rel_path);

            if cfg!(windows) {
                return PathBuf::from(real_path.to_string_lossy().replace('/', "\\"));
            }

            return real_path;
        }
    }

    path.to_owned()
}

/// Convert the provided absolute host path to a virtual guest path suitable
/// for WASI sandboxed runtimes.
pub fn to_virtual_path(
    paths_map: &BTreeMap<PathBuf, PathBuf>,
    path: impl AsRef<Path>,
) -> VirtualPath {
    let path = path.as_ref();

    for (host_path, guest_path) in sort_virtual_paths(paths_map) {
        let mut virtual_path = if path.starts_with(guest_path) {
            path.to_owned()
        } else if let Ok(rel_path) = path.strip_prefix(host_path) {
            guest_path.join(rel_path)
        } else {
            continue;
        };

        // Only forward slashes are allowed in WASI
        if cfg!(windows) {
            virtual_path = PathBuf::from(virtual_path.to_string_lossy().replace('\\', "/"));
        }

        return VirtualPath::WithReal {
            path: virtual_path,
            virtual_prefix: guest_path.to_path_buf(),
            real_prefix: host_path.to_path_buf(),
        };
    }

    VirtualPath::Only(path.to_owned())
}
