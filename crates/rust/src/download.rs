use crate::RustLanguage;
use log::debug;
use proto_core::{async_trait, color, has_command, Describable, Downloadable, ProtoError};
use std::path::{Path, PathBuf};

#[async_trait]
impl Downloadable<'_> for RustLanguage {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.temp_dir.join("download"))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        Ok("https://www.rust-lang.org/tools/install".to_string())
    }

    // Since we don't download Rust for the user, we instead check that `rustup`
    // exists on their machine, as we'll require that command for the install step.
    async fn download(&self, _to_file: &Path, _from_url: Option<&str>) -> Result<bool, ProtoError> {
        debug!(target: self.get_log_target(), "Checking if rustup exists");

        if has_command("rustup") {
            return Ok(true);
        }

        Err(ProtoError::Message(format!(
            "proto requires {} to be installed and available on {} to use Rust. Please install it and try again.",
            color::shell("rustup"),
            color::id("PATH"),
        )))
    }
}
