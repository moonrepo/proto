use crate::error::WarpgateError;
use extism::Manifest;
use miette::IntoDiagnostic;
use reqwest::Url;
use starbase_archive::Archiver;
use starbase_utils::{fs, glob};
use std::path::{Path, PathBuf};
use warpgate_api::VirtualPath;

pub fn extract_prefix_from_slug(slug: &str) -> &str {
    slug.split('/').next().expect("Expected an owner scope!")
}

pub fn extract_suffix_from_slug(slug: &str) -> &str {
    slug.split('/')
        .nth(1)
        .expect("Expected a package or repository name!")
}

pub fn determine_cache_extension(value: &str) -> &str {
    for ext in [".toml", ".json", ".yaml", ".yml"] {
        if value.ends_with(ext) {
            return ext;
        }
    }

    ".wasm"
}

pub fn create_wasm_file_prefix(name: &str) -> String {
    let mut name = name.to_lowercase().replace('-', "_");

    if !name.ends_with("_plugin") {
        name.push_str("_plugin");
    }

    name
}

pub async fn download_from_url_to_file(
    source_url: &str,
    temp_file: &Path,
    client: &reqwest::Client,
) -> miette::Result<()> {
    let url = Url::parse(source_url).into_diagnostic()?;

    // Fetch the file from the HTTP source
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| WarpgateError::Http {
            error,
            url: source_url.to_owned(),
        })?;
    let status = response.status();

    if status.as_u16() == 404 {
        return Err(WarpgateError::DownloadNotFound {
            url: source_url.to_owned(),
        }
        .into());
    }

    if !status.is_success() {
        return Err(WarpgateError::DownloadFailed {
            url: source_url.to_owned(),
            status: status.to_string(),
        }
        .into());
    }

    // Write the bytes to our temporary file
    fs::write_file(
        temp_file,
        response
            .bytes()
            .await
            .map_err(|error| WarpgateError::Http {
                error,
                url: source_url.to_owned(),
            })?,
    )?;

    Ok(())
}

pub fn move_or_unpack_download(temp_file: &Path, dest_file: &Path) -> miette::Result<()> {
    match temp_file.extension().map(|e| e.to_str().unwrap()) {
        // Move these files as-is
        Some("wasm" | "toml" | "json" | "yaml" | "yml") => {
            fs::rename(temp_file, dest_file)?;
        }

        // Unpack archives to temp and move the wasm file
        Some("tar" | "gz" | "xz" | "tgz" | "txz" | "zst" | "zstd" | "zip") => {
            let out_dir = temp_file.parent().unwrap().join("out");

            Archiver::new(&out_dir, dest_file).unpack_from_ext()?;

            let wasm_files = glob::walk_files(&out_dir, ["**/*.wasm"])?;

            if wasm_files.is_empty() {
                return Err(miette::miette!(
                    "No applicable `.wasm` file could be found in downloaded plugin.",
                ));

                // Find a release file first, as some archives include the target folder
            } else if let Some(release_wasm) = wasm_files
                .iter()
                .find(|f| f.to_string_lossy().contains("release"))
            {
                fs::rename(release_wasm, dest_file)?;

                // Otherwise, move the first wasm file available
            } else {
                fs::rename(&wasm_files[0], dest_file)?;
            }

            fs::remove_file(temp_file)?;
            fs::remove_dir_all(out_dir)?;
        }

        Some(x) => {
            return Err(miette::miette!(
                "Unsupported file extension `{}` for downloaded plugin.",
                x
            ));
        }

        None => {
            return Err(miette::miette!(
                "Unsure how to handle downloaded plugin as no file extension/type could be derived."
            ));
        }
    };

    Ok(())
}

/// Convert the provided virtual guest path to an absolute host path.
pub fn from_virtual_path(manifest: &Manifest, path: &Path) -> PathBuf {
    let Some(virtual_paths) = manifest.allowed_paths.as_ref() else {
        return path.to_path_buf();
    };

    for (host_path, guest_path) in virtual_paths {
        if let Ok(rel_path) = path.strip_prefix(guest_path) {
            return host_path.join(rel_path);
        }
    }

    path.to_owned()
}

/// Convert the provided absolute host path to a virtual guest path suitable
/// for WASI sandboxed runtimes.
pub fn to_virtual_path(manifest: &Manifest, path: &Path) -> VirtualPath {
    let Some(virtual_paths) = manifest.allowed_paths.as_ref() else {
        return VirtualPath::Only(path.to_path_buf());
    };

    for (host_path, guest_path) in virtual_paths {
        if let Ok(rel_path) = path.strip_prefix(host_path) {
            let path = guest_path.join(rel_path);

            // Only forward slashes are allowed in WASI
            let path = if cfg!(windows) {
                PathBuf::from(path.to_string_lossy().replace('\\', "/"))
            } else {
                path
            };

            return VirtualPath::WithReal {
                path,
                virtual_prefix: guest_path.to_path_buf(),
                real_prefix: host_path.to_path_buf(),
            };
        }
    }

    VirtualPath::Only(path.to_owned())
}
