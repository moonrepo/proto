use crate::clients::HttpClient;
use crate::loader_error::WarpgateLoaderError;
use base64::prelude::*;
use sha2::{Digest, Sha256};
use starbase_archive::{Archiver, is_supported_archive_extension};
use starbase_utils::{fs, glob, net, net::NetError};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::instrument;
use warpgate_api::{PluginLocator, UrlLocator, VirtualPath};

/// Create a base64 encoded hash based on the provided value.
pub fn hash_base64<T: AsRef<[u8]>>(value: T) -> String {
    BASE64_STANDARD.encode(value)
}

/// Create a SHA256 hash based on the provided value.
pub fn hash_sha256<T: AsRef<[u8]>>(value: T) -> String {
    let mut sha = Sha256::new();
    sha.update(value);

    // Internally bust the cache of plugins
    sha.update("v2");

    format!("{:x}", sha.finalize())
}

/// Determine the extension to use for a cache file, based on our
/// list of supported extensions.
pub fn determine_cache_extension(value: &str) -> Option<&str> {
    [".toml", ".json", ".jsonc", ".yaml", ".yml", ".wasm", ".txt"]
        .into_iter()
        .find(|ext| value.ends_with(ext))
}

/// Attempt to extract a file name from the provided URL,
/// which can be used for caching or temporary file creation.
pub fn extract_file_name_from_url(base: &str) -> String {
    match url::Url::parse(base) {
        Ok(url) => url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .unwrap_or("unknown")
            .into(),
        Err(_) => if let Some(i) = base.rfind('/') {
            &base[i + 1..]
        } else {
            "unknown"
        }
        .into(),
    }
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
        net::DownloadOptions::new(client.create_downloader()),
    )
    .await
    {
        return Err(match error {
            NetError::UrlNotFound { url } => WarpgateLoaderError::NotFound { url },
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
        let out_dir = temp_file.parent().unwrap().join(format!(
            "out-{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ));

        Archiver::new(&out_dir, temp_file).unpack_from_ext()?;

        let wasm_files = glob::walk_files(&out_dir, ["**/*.wasm"])?;

        if wasm_files.is_empty() {
            return Err(WarpgateLoaderError::NoWasmFound {
                path: temp_file.to_path_buf(),
            });
        }

        // Find a release file first, as some archives include the target folder
        if let Some(release_wasm) = wasm_files
            .iter()
            .find(|file| file.iter().any(|comp| comp == "release"))
        {
            fs::copy_file(release_wasm, dest_file)?;
        }
        // Otherwise, move the first wasm file available
        else {
            fs::copy_file(&wasm_files[0], dest_file)?;
        }

        fs::remove_dir_all(out_dir)?;

        return Ok(());
    }

    // Non-archive supported extensions
    match temp_file.extension().and_then(|ext| ext.to_str()) {
        Some("wasm" | "toml" | "json" | "jsonc" | "yaml" | "yml") => {
            // Plugins can be downloaded in parallel, which means
            // that this temp file can also be moved by another process.
            // Because of this, proto constantly runs into "Failed to rename"
            // errors when hitting this block, so let's avoid the failure
            // if the condition is met and assume all is good!
            if temp_file.exists() && !dest_file.exists() {
                fs::copy_file(temp_file, dest_file)?;
            }
        }

        Some(ext) => {
            return Err(WarpgateLoaderError::UnsupportedDownloadExtension {
                ext: ext.to_owned(),
                path: temp_file.to_path_buf(),
            });
        }

        None => {
            return Err(WarpgateLoaderError::UnknownDownloadType {
                path: temp_file.to_path_buf(),
            });
        }
    };

    Ok(())
}

/// Sort virtual paths from longest to shortest host path,
/// so that prefix replacing is deterministic and accurate.
pub fn sort_virtual_paths(paths_list: &mut [(PathBuf, PathBuf)]) {
    paths_list.sort_by(|a, d| d.0.cmp(&a.0).then(d.1.cmp(&a.1)));
}

/// Convert the provided virtual guest path to an absolute host path.
pub fn from_virtual_path(
    paths_list: &[(PathBuf, PathBuf)],
    path: impl AsRef<Path> + Debug,
) -> PathBuf {
    let path = path.as_ref();

    for (host_path, guest_path) in paths_list {
        if let Ok(rel_path) = path.strip_prefix(guest_path) {
            let real_path = host_path.join(rel_path);

            return prepare_from_path(&real_path);
        }
    }

    prepare_from_path(path)
}

/// Convert the provided absolute host path to a virtual guest path suitable
/// for WASI sandboxed runtimes.
pub fn to_virtual_path(
    paths_list: &[(PathBuf, PathBuf)],
    path: impl AsRef<Path> + Debug,
) -> VirtualPath {
    let path = path.as_ref();

    for (host_path, guest_path) in paths_list {
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

#[doc(hidden)]
#[cfg(any(debug_assertions, test))]
pub fn find_debug_locator(name: &str) -> Option<PluginLocator> {
    use crate::test_utils::find_wasm_file_with_name;
    use warpgate_api::FileLocator;

    find_wasm_file_with_name(name).map(|wasm_path| {
        PluginLocator::File(Box::new(FileLocator {
            file: format!("file://{}", wasm_path.display()),
            path: Some(wasm_path),
        }))
    })
}

#[doc(hidden)]
#[cfg(not(any(debug_assertions, test)))]
pub fn find_debug_locator(_name: &str) -> Option<PluginLocator> {
    None
}

#[doc(hidden)]
pub fn find_debug_locator_with_url_fallback(name: &str, version: &str) -> PluginLocator {
    find_debug_locator(name).unwrap_or_else(|| {
        PluginLocator::Url(Box::new(UrlLocator {
            url: format!(
                "https://github.com/moonrepo/plugins/releases/download/{name}-v{version}/{name}.wasm"
            ),
        }))
    })
}
