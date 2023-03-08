use crate::{errors::ProtoError, is_semantic_version};
use std::{fs, path::Path};

#[async_trait::async_trait]
pub trait Detector<'tool>: Send + Sync {
    /// Attempt to detect an applicable version from the provided working directory.
    async fn detect_version_from(&self, _working_dir: &Path) -> Result<Option<String>, ProtoError> {
        Ok(None)
    }
}

pub fn load_version_file(path: &Path) -> Result<String, ProtoError> {
    Ok(fs::read_to_string(path)
        .map_err(|e| ProtoError::Fs(path.to_path_buf(), e.to_string()))?
        .trim()
        .to_owned())
}

pub fn get_fixed_version(version: &str) -> Option<String> {
    if version.starts_with('^')
        || version.starts_with('~')
        || version.starts_with('>')
        || version.starts_with('<')
        || version.contains(' ')
        || version.contains('|')
    {
        return None;
    }

    let maybe_semver = if let Some(value) = version.strip_prefix('=') {
        value
    } else {
        version
    };

    let maybe_semver = &maybe_semver.replace(".*", "");

    if is_semantic_version(maybe_semver) {
        return Some(maybe_semver.to_owned());
    }

    None
}
