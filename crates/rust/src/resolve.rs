use crate::RustLanguage;
use proto_core::{
    async_trait, create_version_manifest_from_tags, is_offline, is_semantic_version, load_git_tags,
    remove_v_prefix, ProtoError, Resolvable, Tool, VersionManifest,
};
use tracing::debug;

#[async_trait]
impl Resolvable<'_> for RustLanguage {
    fn get_resolved_version(&self) -> &str {
        match self.version.as_ref() {
            Some(version) => version,
            None => "stable",
        }
    }

    async fn load_version_manifest(&self) -> Result<VersionManifest, ProtoError> {
        let tags = load_git_tags("https://github.com/rust-lang/rust")
            .await?
            .into_iter()
            .filter(|t| !t.ends_with("^{}"))
            .collect::<Vec<_>>();

        let mut manifest = create_version_manifest_from_tags(tags);

        manifest.inherit_aliases(&self.get_manifest()?.aliases);

        Ok(manifest)
    }

    async fn resolve_version(&mut self, initial_version: &str) -> Result<String, ProtoError> {
        if let Some(version) = &self.version {
            return Ok(version.to_owned());
        }

        let initial_version = remove_v_prefix(initial_version).to_lowercase();

        // If offline but we have a fully qualified semantic version,
        // exit early and assume the version is legitimate
        if is_semantic_version(&initial_version) && is_offline() {
            self.set_version(&initial_version);

            return Ok(initial_version);
        }

        debug!("Resolving a semantic version for \"{}\"", initial_version);

        let manifest = self.load_version_manifest().await?;

        let candidate = if initial_version == "stable"
            || initial_version == "beta"
            || initial_version == "nightly"
        {
            debug!("Using channel {}", initial_version);

            &initial_version
        } else {
            let candidate = manifest.find_version(&initial_version)?;

            debug!("Resolved to {}", candidate);

            candidate
        };

        self.set_version(candidate);

        Ok(candidate.to_owned())
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
