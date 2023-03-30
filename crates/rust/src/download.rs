use crate::platform::RustArch;
use crate::RustLanguage;
use proto_core::{async_trait, Downloadable, ProtoError, Resolvable};
use std::env::consts;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProtoError> {
    let arch = RustArch::from_os_arch()?;

    if !matches!(arch, RustArch::Amd64 | RustArch::Arm64) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Go".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("go{version}.darwin-{arch}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn get_archive_file_path(version: &str) -> Result<String, ProtoError> {
    let arch = RustArch::from_os_arch()?;

    if !matches!(
        arch,
        RustArch::I386 | RustArch::Amd64 | RustArch::Arm64 | RustArch::Armv6l | RustArch::S390x
    ) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Go".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("go{version}.linux-{arch}"))
}

#[cfg(target_os = "windows")]
pub fn get_archive_file_path(version: &str) -> Result<String, ProtoError> {
    let arch = RustArch::from_os_arch()?;

    if !matches!(arch, RustArch::I386 | RustArch::Amd64 | RustArch::Arm64) {
        return Err(ProtoError::UnsupportedArchitecture(
            "Go".into(),
            arch.to_string(),
        ));
    }

    Ok(format!("go{version}.windows-{arch}"))
}

pub fn get_archive_file(version: &str) -> Result<String, ProtoError> {
    let ext = if consts::OS == "windows" {
        "zip"
    } else {
        "tar.gz"
    };

    Ok(format!("{}.{}", get_archive_file_path(version)?, ext))
}

#[async_trait]
impl Downloadable<'_> for RustLanguage {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(get_archive_file(self.get_resolved_version())?))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        let version = self.get_resolved_version();

        let version = match version.strip_suffix(".0") {
            Some(s) => s,
            None => version,
        };

        Ok(format!(
            "https://dl.google.com/go/{}",
            get_archive_file(version)?
        ))
    }
}
