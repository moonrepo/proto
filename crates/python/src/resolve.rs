use crate::PythonLanguage;
use proto_core::{async_trait, ProtoError, Resolvable, VersionManifest, VersionManifestEntry};
use std::collections::BTreeMap;

#[async_trait]
impl Resolvable<'_> for PythonLanguage {
    fn get_resolved_version(&self) -> &str {
        match self.version.as_ref() {
            Some(version) => version,
            None => "latest",
        }
    }

    async fn load_version_manifest(&self) -> Result<VersionManifest, ProtoError> {
        // https://api.github.com/repos/indygreg/python-build-standalone/releases
        // lets just hard code latest while I learn
        // eventually use same strategy as rye with above link
        // or we could use CLI directly to list available versions
        // `rye toolchain list --include-downloadable`

        let mut aliases = BTreeMap::new();
        let mut versions = BTreeMap::new();

        versions.insert(
            "3.11.3".into(),
            VersionManifestEntry {
                alias: None,
                version: "3.11.3".into(),
            },
        );
        aliases.insert("latest".into(), "3.11.3".into());

        let manifest = VersionManifest { aliases, versions };
        Ok(manifest)
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
