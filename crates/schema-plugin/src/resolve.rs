use crate::SchemaPlugin;
use proto_core::{
    async_trait, create_version_manifest_from_tags, load_git_tags, load_versions_manifest,
    remove_v_prefix, Describable, ProtoError, Resolvable, Tool, Version, VersionManifest,
    VersionManifestEntry,
};
use starbase_utils::json::JsonValue;
use std::collections::BTreeMap;

#[async_trait]
impl Resolvable<'_> for SchemaPlugin {
    fn get_resolved_version(&self) -> &str {
        match self.version.as_ref() {
            Some(version) => version,
            None => "latest",
        }
    }

    async fn load_version_manifest(&self) -> Result<VersionManifest, ProtoError> {
        let mut manifest =

        // From git tags
        if let Some(git_url) = &self.schema.resolve.git_url {
            let tag_pattern = regex::Regex::new(&self.schema.resolve.git_tag_pattern).unwrap();
            let tags = load_git_tags(git_url)
                .await?
                .into_iter()
                .filter_map(|t| {
                    tag_pattern.captures(&t)
                        .map(|captures| remove_v_prefix(captures.get(1).unwrap().as_str()))
                })
                .collect::<Vec<_>>();

            create_version_manifest_from_tags(tags)

        // From manifest JSON response
        } else if let Some(manifest_url) = &self.schema.resolve.manifest_url {
            let response: Vec<JsonValue> = load_versions_manifest(manifest_url).await?;
            let version_key = &self.schema.resolve.manifest_version_key;
            let mut aliases = BTreeMap::new();
            let mut versions = BTreeMap::new();

            let mut add_version = |v: &str| {
                if let Ok(version) = Version::parse(&remove_v_prefix(v)) {
                    let entry = VersionManifestEntry {
                        alias: None,
                        version: version.to_string(),
                    };

                    aliases.entry("latest".to_owned()).and_modify(|v| {
                        *v = version.to_string();
                    }).or_insert(version.to_string());

                    versions.insert(version.to_string(), entry);
                }
            };

            for row in response {
                match row {
                    JsonValue::String(v) => {
                        add_version(&v);
                    },
                    JsonValue::Object(o) => {
                        if let Some(JsonValue::String(v)) = o.get(version_key) {
                            add_version(v);
                        }
                    },
                    _ => {},
                }
            }

            VersionManifest {
                aliases,
                versions,
            }

            // Invalid schema
        } else {
            return Err(ProtoError::Message(format!("Unable to resolve versions for {}. Schema either requires a `git_url` or `manifest_url`.", self.get_name())));
        };

        manifest.inherit_aliases(&self.get_manifest()?.aliases);

        Ok(manifest)
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
