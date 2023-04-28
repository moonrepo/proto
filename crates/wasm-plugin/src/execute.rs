use crate::WasmPlugin;
use proto_core::{async_trait, Describable, Executable, ProtoError};
use std::path::{Path, PathBuf};

#[async_trait]
impl Executable<'_> for WasmPlugin {
    async fn find_bin_path(&mut self) -> Result<(), ProtoError> {
        self.bin_path = Some(PathBuf::from("wasm-plugin")); // TODO
        Ok(())
    }

    fn get_bin_path(&self) -> Result<&Path, ProtoError> {
        match self.bin_path.as_ref() {
            Some(bin) => Ok(bin),
            None => Err(ProtoError::MissingTool(self.get_name())),
        }
    }

    fn get_globals_bin_dir(&self) -> Result<PathBuf, ProtoError> {
        Ok(PathBuf::new())
    }
}
