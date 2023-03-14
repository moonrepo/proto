use crate::depman::NodeDependencyManager;
use crate::platform::PackageJson;
use proto_core::{async_trait, detect_fixed_version, Detector, ProtoError, Tool};
use std::path::Path;

// https://nodejs.org/api/packages.html#packagemanager
#[async_trait]
impl Detector<'_> for NodeDependencyManager {
    async fn detect_version_from(&self, working_dir: &Path) -> Result<Option<String>, ProtoError> {
        let package_path = working_dir.join("package.json");

        if package_path.exists() {
            let package_json = PackageJson::load(&package_path)?;

            if let Some(manager) = package_json.package_manager {
                let mut parts = manager.split('@');
                let name = parts.next().unwrap_or_default();

                if name == self.package_name {
                    return Ok(Some(parts.next().unwrap_or("latest").to_owned()));
                }
            }

            if let Some(engines) = package_json.engines {
                if let Some(constraint) = engines.get(&self.package_name) {
                    return detect_fixed_version(constraint, self.get_manifest_path()?);
                }
            }
        }

        Ok(None)
    }
}
