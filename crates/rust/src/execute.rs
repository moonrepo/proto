use crate::RustLanguage;
use proto_core::{async_trait, get_home_dir, Describable, Executable, ProtoError};
use std::{
    env,
    path::{Path, PathBuf},
};

#[async_trait]
impl Executable<'_> for RustLanguage {
    async fn find_bin_path(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }

    fn get_bin_path(&self) -> Result<&Path, ProtoError> {
        Err(ProtoError::MissingTool(self.get_name()))
    }

    fn get_globals_bin_dir(&self) -> Result<PathBuf, ProtoError> {
        let root = if let Ok(root) = env::var("CARGO_INSTALL_ROOT") {
            PathBuf::from(root)
        } else if let Ok(root) = env::var("CARGO_HOME") {
            PathBuf::from(root)
        } else {
            get_home_dir()?.join(".cargo")
        };

        Ok(root.join("bin"))
    }
}
