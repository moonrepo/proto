use crate::NodeLanguage;
use log::debug;
use proto_core::{
    async_trait, is_offline, is_semantic_version, load_versions_manifest, parse_version,
    remove_v_prefix, Describable, ProtoError, Resolvable, VersionManifest, VersionManifestEntry,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(untagged)]
enum NodeLTS {
    Name(String),
    State(bool),
}

#[derive(Deserialize)]
struct NodeDistVersion {
    lts: NodeLTS,
    version: String, // Starts with v
}

#[async_trait]
impl Resolvable<'_> for NodeLanguage {
    fn get_resolved_version(&self) -> &str {
        match self.version.as_ref() {
            Some(version) => version,
            None => "latest",
        }
    }

    async fn load_version_manifest(&self) -> Result<VersionManifest, ProtoError> {
        let mut aliases = BTreeMap::new();
        let mut versions = BTreeMap::new();
        let response: Vec<NodeDistVersion> =
            load_versions_manifest("https://nodejs.org/dist/index.json").await?;

        for (index, item) in response.iter().enumerate() {
            // First item is always the latest
            if index == 0 {
                aliases.insert("latest".into(), item.version.clone());
            }

            let mut entry = VersionManifestEntry {
                alias: None,
                version: remove_v_prefix(&item.version),
            };

            if let NodeLTS::Name(alias) = &item.lts {
                let alias = alias.to_lowercase();

                // The first encounter of an lts in general is the latest stable
                if !aliases.contains_key("stable") {
                    aliases.insert("stable".into(), item.version.clone());
                }

                // The first encounter of an lts is the latest version for that alias
                if !aliases.contains_key(&alias) {
                    aliases.insert(alias.clone(), item.version.clone());
                }

                entry.alias = Some(alias);
            }

            versions.insert(entry.version.clone(), entry);
        }

        Ok(VersionManifest { aliases, versions })
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

        debug!(
            target: self.get_log_target(),
            "Resolving a semantic version for \"{}\"",
            initial_version,
        );

        let manifest = self.load_version_manifest().await?;
        let candidate;

        // Latest version is always at the top
        if initial_version == "node" || initial_version == "latest" {
            candidate = manifest.find_version_from_alias("latest")?;

        // Stable version is the first with an LTS
        } else if initial_version == "stable"
            || initial_version == "lts-*"
            || initial_version == "lts/*"
        {
            candidate = manifest.find_version_from_alias("stable")?;

            // Find the first version with a matching LTS
        } else if initial_version.starts_with("lts-") || initial_version.starts_with("lts/") {
            candidate = manifest.find_version_from_alias(&initial_version[4..])?;

            // Either an alias or version
        } else {
            candidate = manifest.find_version(&initial_version)?;
        }

        let version = parse_version(candidate)?.to_string();

        debug!(target: self.get_log_target(), "Resolved to {}", version);

        self.set_version(&version);

        Ok(version)
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
