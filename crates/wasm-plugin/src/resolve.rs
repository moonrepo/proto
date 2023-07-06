use crate::WasmPlugin;
use proto_core::{
    async_trait, is_offline, is_semantic_version, remove_v_prefix, Describable, ProtoError,
    Resolvable, Tool, VersionManifest, VersionManifestEntry,
};
use proto_pdk_api::{LoadVersionsInput, LoadVersionsOutput, ResolveVersionInput, ResolveVersionOutput};
use tracing::debug;

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

    async fn resolve_version(&mut self, initial_version: &str) -> Result<String, ProtoError> {
        if self.get_resolved_version() != "latest" {
            return Ok(self.get_resolved_version().to_owned());
        }

        let initial_version = remove_v_prefix(initial_version).to_lowercase();

        // If offline but we have a fully qualified semantic version,
        // exit early and assume the version is legitimate
        if is_semantic_version(&initial_version) && is_offline() {
            self.set_version(&initial_version);

            return Ok(initial_version);
        }

        debug!(
            tool = self.get_id(),
            initial_version = initial_version,
            "Resolving a semantic version for \"{}\"",
            initial_version
        );

        let manifest = self.load_version_manifest().await?;
        let mut version = "";

        if self.has_func("resolve_version") {
            let resolved: ResolveVersionOutput = self.call_func_with(
                "resolve_version",
                ResolveVersionInput {
                    initial: initial_version.to_owned(),
                    env: self.get_environment()?,
                },
            )?;

            if let Some(candidate) = resolved.candidate {
                version = manifest.find_version(candidate)?;
            }
        }

        if version.is_empty() {
            version = manifest.find_version(&initial_version)?;
        }

        debug!(tool = self.get_id(), version, "Resolved to {}", version);

        self.set_version(version);

        Ok(version.to_owned())
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
