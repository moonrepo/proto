use crate::platform::DenoArch;
use crate::DenoLanguage;
use proto_core::{async_trait, Downloadable, ProtoError, Resolvable};
use std::path::PathBuf;

#[cfg(target_os = "macos")]
pub fn get_archive_file_path() -> Result<String, ProtoError> {
    let arch = DenoArch::from_os_arch()?;

    if !matches!(arch, DenoArch::X64 | DenoArch::Arm64) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Deno".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("deno-{arch}-apple-darwin"))
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn get_archive_file_path() -> Result<String, ProtoError> {
    let arch = DenoArch::from_os_arch()?;

    if !matches!(arch, DenoArch::X64) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Deno".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("deno-{arch}-unknown-linux-gnu"))
}

#[cfg(target_os = "windows")]
pub fn get_archive_file_path() -> Result<String, ProtoError> {
    let arch = DenoArch::from_os_arch()?;

    if !matches!(arch, DenoArch::X64) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Deno".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("deno-{arch}-pc-windows-msvc"))
}

pub fn get_archive_file() -> Result<String, ProtoError> {
    Ok(format!("{}.{}", get_archive_file_path()?, "zip"))
}

#[async_trait]
impl Downloadable<'_> for DenoLanguage {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.temp_dir.join(format!(
            "{}-{}",
            self.get_resolved_version(),
            get_archive_file()?
        )))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        Ok(format!(
            "https://github.com/denoland/deno/releases/download/v{}/{}",
            self.get_resolved_version(),
            get_archive_file()?
        ))
    }
}
