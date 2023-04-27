use crate::GoLanguage;
use proto_core::{
    async_trait, create_version_manifest_from_tags, load_git_tags, ProtoError, Resolvable, Tool,
    Version, VersionManifest,
};

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

        manifest.inherit_aliases(&self.get_manifest()?.aliases);

        Ok(manifest)
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
