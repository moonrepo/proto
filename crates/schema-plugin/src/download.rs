use crate::SchemaPlugin;
use proto_core::{async_trait, Describable, Downloadable, ProtoError, Resolvable};
use std::path::PathBuf;

#[async_trait]
impl Downloadable<'_> for SchemaPlugin {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.temp_dir.join(format!(
            "v{}-{}{}",
            self.get_resolved_version(),
            self.get_bin_name(),
            self.schema.get_download_ext()
        )))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        Ok(self.format_string(&self.schema.install.download_url))
    }
}
