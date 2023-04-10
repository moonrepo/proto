use crate::GoLanguage;
use proto_core::{
    async_trait, create_version_manifest_from_tags, is_offline, is_semantic_version, load_git_tags,
    remove_v_prefix, Manifest, ProtoError, Resolvable, Tool, Version, VersionManifest,
};
use tracing::debug;

trait BaseVersion {
    fn base_version(&self) -> String;
}

impl<'a> BaseVersion for Version<'a> {
    fn base_version(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

#[async_trait]
impl Resolvable<'_> for GoLanguage {
    fn get_resolved_version(&self) -> &str {
        match self.version.as_ref() {
            Some(version) => version,
            None => "latest",
        }
    }

    // https://go.dev/dl/?mode=json&include=all
    async fn load_version_manifest(&self) -> Result<VersionManifest, ProtoError> {
        let tags = load_git_tags("https://github.com/golang/go")
            .await?
            .iter()
            .filter(|t| t.starts_with("go"))
            .map(|t| {
                t.strip_prefix("go")
                    .unwrap()
                    // go1.4rc1, go1.19beta, etc
                    .replace("alpha", ".0-alpha")
                    .replace("beta", ".0-beta")
                    .replace("rc", ".0-rc")
            })
            .collect::<Vec<_>>();

        let mut manifest = create_version_manifest_from_tags(tags);

        manifest.inherit_aliases(&Manifest::load(self.get_manifest_path())?.aliases);

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

        let candidate = if initial_version.contains("rc") || initial_version.contains("beta") {
            manifest.get_version(&initial_version)?
        } else {
            manifest.find_version(&initial_version)?
        };

        debug!("Resolved to {}", candidate);

        self.set_version(candidate);

        Ok(candidate.to_owned())
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
