use crate::loader_error::WarpgateLoaderError;
use base64::prelude::*;
use sha2::{Digest, Sha256};
use starbase_archive::{Archiver, is_supported_archive_extension};
use starbase_utils::net::{self, DownloadOptions, NetError};
use starbase_utils::{fs, glob};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
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
#[instrument(skip(options))]
pub async fn download_from_url_to_file(
    source_url: &str,
    dest_file: &Path,
    options: DownloadOptions,
) -> Result<(), WarpgateLoaderError> {
    if let Err(error) = net::download_from_url_with_options(source_url, dest_file, options).await {
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

fn find_file_with_extension(paths: &[PathBuf], ext: &str) -> Option<PathBuf> {
    // Find a release file first, as some archives include the target folder
    if ext == "wasm"
        && let Some(release) = paths.iter().find(|path| {
            path.extension().is_some_and(|e| e == "wasm")
                && path.iter().any(|comp| comp == "release")
        })
    {
        return Some(release.to_path_buf());
    }

    // Otherwise, find the first file available
    paths
        .iter()
        .find(|path| path.extension().is_some_and(|e| e == ext))
        .cloned()
}

/// If the temporary file is an archive, unpack it into the destination,
/// otherwise move the file into the destination.
#[instrument]
pub fn move_or_unpack_file(
    temp_file: &Path,
    dest_file: &mut PathBuf,
    extensions: &[String],
) -> Result<(), WarpgateLoaderError> {
    // The temporary file is an archive that may contain a plugin/wasm file,
    // so we need to unpack it and find the plugin file inside!
    if is_supported_archive_extension(temp_file) {
        let mut out_dir = temp_file.to_path_buf();

        // Unpack the archive into a temporary directory, using the same
        // name as the file, so we can easily reference it if needed
        out_dir.set_file_name(
            temp_file
                .file_prefix()
                .and_then(|prefix| prefix.to_str())
                .unwrap_or("out"),
        );

        Archiver::new(&out_dir, temp_file).unpack_from_ext()?;

        let ext_glob = format!("**/*.{{{}}}", extensions.join(","));
        let files = glob::walk_files(&out_dir, [&ext_glob])?;

        if files.is_empty() {
            return Err(WarpgateLoaderError::NoWasmFound {
                path: temp_file.to_path_buf(),
            });
        } else {
            for ext in extensions {
                if let Some(file) = find_file_with_extension(&files, ext) {
                    // At this point, we know what type of file to use,
                    // so update the destination extension to match the file we found,
                    // otherwise "wasm" will be used for non-wasm files!
                    dest_file.set_extension(ext);

                    fs::copy_file(file, dest_file)?;

                    break;
                }
            }
        }

        fs::remove_dir_all(out_dir)?;

        return Ok(());
    }

    // Non-archive supported extensions, typically the plugin file itself.
    // We extract the extension from the destination file, as that path is
    // the source of truth for the plugin file type, while the temporary
    // file is extensionless (when not an archive).
    match dest_file.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => {
            if extensions.iter().any(|e| e == ext) {
                // Plugins can be downloaded in parallel, which means
                // that this temp file can also be moved by another process.
                // Because of this, proto constantly runs into "Failed to rename"
                // errors when hitting this block, so let's avoid the failure
                // if the condition is met and assume all is good!
                if temp_file.exists() && !dest_file.exists() {
                    fs::rename(temp_file, dest_file)?;
                }
            } else {
                return Err(WarpgateLoaderError::UnsupportedDownloadExtension {
                    ext: ext.to_owned(),
                    path: temp_file.to_path_buf(),
                });
            }
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
