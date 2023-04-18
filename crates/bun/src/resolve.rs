use crate::BunLanguage;
use core::str;
use proto_core::{
    async_trait, create_version_manifest_from_tags, load_git_tags, Manifest, ProtoError,
    Resolvable, Tool, VersionManifest,
};

#[async_trait]
impl Resolvable<'_> for BunLanguage {
    fn get_resolved_version(&self) -> &str {
        match self.version.as_ref() {
            Some(version) => version,
            None => "latest",
        }
    }

    async fn load_version_manifest(&self) -> Result<VersionManifest, ProtoError> {
        let tags = load_git_tags("https://github.com/oven-sh/bun")
            .await?
            .iter()
            .filter(|t| t.starts_with("bun-v"))
            .map(|t| t.strip_prefix("bun-v").unwrap().to_owned())
            .collect::<Vec<_>>();

        let mut manifest = create_version_manifest_from_tags(tags);

        manifest.inherit_aliases(&Manifest::load(self.get_manifest_path())?.aliases);

        Ok(manifest)
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
