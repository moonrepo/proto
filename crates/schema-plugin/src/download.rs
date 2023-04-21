use crate::SchemaPlugin;
use proto_core::{async_trait, Downloadable, ProtoError, Resolvable};
use std::path::PathBuf;

#[async_trait]
impl Downloadable<'_> for SchemaPlugin {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(self.get_resolved_version())
            .join(self.get_download_file()?))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        Ok(self
            .interpolate_tokens(&self.schema.install.download_url)
            .replace("{download_file}", &self.get_download_file()?))
    }
}
