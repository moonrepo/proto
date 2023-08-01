use crate::WasmPlugin;
use proto_core::{async_trait, Downloadable, ProtoError, Resolvable};
use std::path::PathBuf;

#[async_trait]
impl Downloadable<'_> for WasmPlugin {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        let params = self.get_install_params()?;

        let name = match &params.download_name {
            Some(file) => file.to_owned(),
            None => {
                let url = url::Url::parse(&params.download_url).map_err(|e| {
                    ProtoError::Message(format!("Failed to parse download URL: {e}"))
                })?;

                url.path_segments().unwrap().last().unwrap().to_owned()
            }
        };

        Ok(self.temp_dir.join(self.get_resolved_version()).join(name))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        Ok(self.get_install_params()?.download_url)
    }

    fn should_skip_download(&self) -> Result<bool, ProtoError> {
        Ok(self.get_install_params()?.skip_download)
    }
}
