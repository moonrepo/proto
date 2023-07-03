use crate::PythonLanguage;
use proto_core::{async_trait, color, has_command, Describable, Downloadable, ProtoError};
use std::path::{Path, PathBuf};
use tracing::debug;

#[async_trait]
impl Downloadable<'_> for PythonLanguage {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.temp_dir.join("download"))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        Ok("https://rye-up.com/guide/installation".to_string())
    }

    // Since we don't download Python for the user, we instead check that `rye`
    // exists on their machine, as we'll require that command for the install step.
    async fn download(&self, _to_file: &Path, _from_url: Option<&str>) -> Result<bool, ProtoError> {
        debug!(tool = self.get_id(), "Checking if rye exists");

        if has_command("rye") {
            return Ok(true);
        }

        Err(ProtoError::Message(format!(
            "proto requires {} to be installed and available on {} to use Rust. Please install it and try again.",
            color::shell("rye"),
            color::id("PATH"),
        )))
    }
}
