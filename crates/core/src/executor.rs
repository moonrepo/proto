use crate::errors::ProtoError;
use std::path::{Path, PathBuf};

#[async_trait::async_trait]
pub trait Executable<'tool>: Send + Sync {
    /// Find the absolute file path to the tool's binary that will be executed.
    /// This happens after a tool has been downloaded and installed.
    async fn find_bin_path(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }

    /// Return an absolute file path to the executable binary for the tool.
    fn get_bin_path(&self) -> Result<&Path, ProtoError>;

    /// Return an absolute file path to the directory containing all
    /// globally installed packages.
    fn get_globals_bin_dir(&self) -> Result<PathBuf, ProtoError>;
}

#[cfg(target_os = "windows")]
pub fn get_bin_name(name: &str) -> String {
    format!("{}.exe", name)
}

#[cfg(not(target_os = "windows"))]
pub fn get_bin_name(name: &str) -> String {
    name.to_string()
}
