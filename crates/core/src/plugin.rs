use crate::{download_from_url, errors::ProtoError, get_plugins_dir};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tracing::trace;

#[tracing::instrument(skip_all)]
pub async fn download_plugin<P, U>(name: P, url: U) -> Result<PathBuf, ProtoError>
where
    P: AsRef<str>,
    U: AsRef<str>,
{
    let url = url.as_ref();
    let mut sha = Sha256::new();
    sha.update(url.as_bytes());

    let mut file_name = format!("{}-{:x}", name.as_ref(), sha.finalize());

    if url.ends_with(".wasm") {
        file_name.push_str(".wasm");
    } else if url.ends_with(".toml") {
        file_name.push_str(".toml");
    }

    let plugin_path = get_plugins_dir()?.join(file_name);

    if !plugin_path.exists() {
        trace!(
            plugin = name.as_ref(),
            "Plugin does not exist in cache, attempting to download"
        );

        download_from_url(url, &plugin_path).await?;
    }

    Ok(plugin_path)
}
