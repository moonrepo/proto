use crate::platform::BunArch;
use crate::BunLanguage;
use proto_core::{async_trait, Downloadable, ProtoError, Resolvable};
use std::path::PathBuf;

#[cfg(target_os = "macos")]
pub fn get_archive_file_path() -> Result<String, ProtoError> {
    let arch = BunArch::from_os_arch()?;

    if !matches!(arch, BunArch::X64 | BunArch::Arm64) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Bun".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("bun-darwin-{arch}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn get_archive_file_path() -> Result<String, ProtoError> {
    let arch = BunArch::from_os_arch()?;

    if !matches!(arch, BunArch::X64 | BunArch::Arm64) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Bun".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("bun-linux-{arch}"))
}

#[cfg(target_os = "windows")]
pub fn get_archive_file_path() -> Result<String, ProtoError> {
    let arch = BunArch::from_os_arch()?;

    return Err(ProtoError::UnsupportedArchitecture(
        "Bun".into(),
        arch.to_string(),
    ));
}

pub fn get_archive_file() -> Result<String, ProtoError> {
    Ok(format!("{}.{}", get_archive_file_path()?, "zip"))
}

#[async_trait]
impl Downloadable<'_> for BunLanguage {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.temp_dir.join(format!(
            "v{}-{}",
            self.get_resolved_version(),
            get_archive_file()?
        )))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        Ok(format!(
            "https://github.com/oven-sh/bun/releases/download/bun-v{}/{}",
            self.get_resolved_version(),
            get_archive_file()?
        ))
    }
}
