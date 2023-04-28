use crate::WasmPlugin;
use proto_core::{async_trait, ProtoError, Resolvable, VersionManifest};

#[async_trait]
impl Resolvable<'_> for WasmPlugin {
    fn get_resolved_version(&self) -> &str {
        match self.version.as_ref() {
            Some(version) => version,
            None => "latest",
        }
    }

    async fn load_version_manifest(&self) -> Result<VersionManifest, ProtoError> {
        // let result = self.call_method("hello_world", "{}");

        // dbg!(&result);

        Ok(VersionManifest::default())
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
