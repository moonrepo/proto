use crate::WasmPlugin;
use proto_core::{
    async_trait, ProtoError, Resolvable, Tool, VersionManifest, VersionManifestEntry,
};
use proto_pdk::{LoadVersionsInput, LoadVersionsOutput};

#[async_trait]
impl Resolvable<'_> for WasmPlugin {
    fn get_resolved_version(&self) -> &str {
        match self.version.as_ref() {
            Some(version) => version,
            None => "latest",
        }
    }

    async fn load_version_manifest(&self) -> Result<VersionManifest, ProtoError> {
        let mut available: LoadVersionsOutput = self.cache_func_with(
            "load_versions",
            LoadVersionsInput {
                env: self.get_environment()?,
            },
        )?;

        available.versions.sort_by(|a, d| d.cmp(a));
        available.canary_versions.sort_by(|a, d| d.cmp(a));

        let mut manifest = VersionManifest::default();

        for (alias, version) in available.aliases {
            manifest.aliases.insert(alias, version.to_string());
        }

        manifest.aliases.insert(
            "latest".into(),
            available
                .latest
                .unwrap_or_else(|| available.versions[0].clone())
                .to_string(),
        );

        for version in available.versions {
            manifest.versions.insert(
                version.to_string(),
                VersionManifestEntry {
                    alias: None,
                    version: version.to_string(),
                },
            );
        }

        manifest.inherit_aliases(&self.get_manifest()?.aliases);

        Ok(manifest)
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
