use crate::WasmPlugin;
use proto_core::{async_trait, Downloadable, ProtoError, Resolvable};
use proto_pdk::InstallParams;
use std::path::PathBuf;

#[async_trait]
impl Downloadable<'_> for WasmPlugin {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        let params: InstallParams =
            self.cache_func_with("register_install_params", self.get_env_input())?;

        let filename = match &params.download_file {
            Some(file) => file.to_owned(),
            None => {
                let url = url::Url::parse(&params.download_url).map_err(|e| {
                    ProtoError::Message(format!("Failed to parse download URL: {e}"))
                })?;

                url.path_segments().unwrap().last().unwrap().to_owned()
            }
        };

        Ok(self
            .temp_dir
            .join(self.get_resolved_version())
            .join(filename))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        let params: InstallParams =
            self.cache_func_with("register_install_params", self.get_env_input())?;

        Ok(params.download_url)
    }
}
