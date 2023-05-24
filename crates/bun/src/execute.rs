use crate::BunLanguage;
use proto_core::{
    async_trait, get_bin_name, get_home_dir, Describable, Executable, Installable, ProtoError,
};
use std::path::{Path, PathBuf};

#[async_trait]
impl Executable<'_> for BunLanguage {
    async fn find_bin_path(&mut self) -> Result<(), ProtoError> {
        let bin_path = self.get_install_dir()?.join(get_bin_name("bun"));

        if bin_path.exists() {
            self.bin_path = Some(bin_path);
        } else {
            return Err(ProtoError::ExecuteMissingBin(self.get_name(), bin_path));
        }

        Ok(())
    }

    fn get_bin_path(&self) -> Result<&Path, ProtoError> {
        match self.bin_path.as_ref() {
            Some(bin) => Ok(bin),
            None => Err(ProtoError::MissingTool(self.get_name())),
        }
    }

    fn get_globals_bin_dir(&self) -> Result<PathBuf, ProtoError> {
        Ok(get_home_dir()?.join(".bun").join("bin"))
    }
}
