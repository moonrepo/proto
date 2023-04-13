use crate::SchemaPlugin;
use proto_core::{async_trait, Downloadable, ProtoError, Resolvable};
use std::env::consts;
use std::path::PathBuf;

#[async_trait]
impl Downloadable<'_> for SchemaPlugin {
    fn get_download_path(&self) -> Result<PathBuf, ProtoError> {
        let parts = self.get_download_url()?.split('/');

        Ok(self.temp_dir.join(format!(
            "v{}-{}",
            self.get_resolved_version(),
            parts.last().unwrap()
        )))
    }

    fn get_download_url(&self) -> Result<String, ProtoError> {
        Ok(self
            .schema
            .install
            .download_url
            .replace("{version}", self.get_resolved_version())
            .replace("{arch}", self.schema.get_arch())
            .replace("{os}", self.schema.get_os())
            .replace(
                "{ext}",
                self.schema.install.download_ext.get(consts::OS).or_else(""),
            ))
    }
}
