use crate::platform::PackageJson;
use crate::NodeLanguage;
use proto_core::{async_trait, get_fixed_version, load_version_file, Detector, ProtoError};
use std::path::Path;

#[async_trait]
impl Detector<'_> for NodeLanguage {
    async fn detect_version_from(&self, working_dir: &Path) -> Result<Option<String>, ProtoError> {
        let nvmrc = working_dir.join(".nvmrc");

        if nvmrc.exists() {
            return Ok(Some(load_version_file(&nvmrc)?));
        }

        let nodenv = working_dir.join(".node-version");

        if nodenv.exists() {
            return Ok(Some(load_version_file(&nodenv)?));
        }

        let package_path = working_dir.join("package.json");

        if package_path.exists() {
            let package_json = PackageJson::load(&package_path)?;

            if let Some(engines) = package_json.engines {
                if let Some(constraint) = engines.get("node") {
                    return Ok(get_fixed_version(constraint));
                }
            }
        }

        Ok(None)
    }
}
