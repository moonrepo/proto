use crate::GoLanguage;
use core::str;
use log::debug;
use proto_core::{
    async_trait, is_offline, is_semantic_version, load_git_tags, remove_v_prefix, Describable,
    ProtoError, Resolvable, Version, VersionManifest, VersionManifestEntry,
};
use std::collections::BTreeMap;

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
        let v = self.version.as_ref().unwrap();
        match v.strip_suffix(".0") {
            Some(s) => s,
            None => v,
        }
    }

    async fn load_manifest(&self) -> Result<VersionManifest, ProtoError> {
        let mut alias_max = BTreeMap::new();
        let mut latest = Version::new(0, 0, 0);

        let mut aliases = BTreeMap::new();
        let mut versions = BTreeMap::new();

        let tags = load_git_tags("https://github.com/golang/go").await?;

        for tag in &tags {
            if !tag.starts_with("go") {
                continue;
            }

            let ver_str = tag.strip_prefix("go").unwrap();

            if let Ok(ver) = Version::parse(ver_str) {
                let entry = VersionManifestEntry {
                    alias: None,
                    version: String::from(ver_str),
                };
                let base_version = ver.base_version();

                if latest < ver {
                    latest = ver.clone();
                }

                let current: Option<&Version> = alias_max.get(&base_version);

                match current {
                    Some(current_version) => {
                        if current_version < &ver {
                            aliases.insert(base_version.clone(), entry.version.clone());
                            alias_max.insert(base_version, ver);
                        }
                    }
                    None => {
                        aliases.insert(base_version.clone(), entry.version.clone());
                        alias_max.insert(base_version, ver);
                    }
                }

                versions.insert(entry.version.clone(), entry);
            }
        }

        aliases.insert("latest".into(), latest.to_string());

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

        let manifest = self.load_manifest().await?;
        let candidate = if initial_version.contains("rc") || initial_version.contains("beta") {
            manifest.get_version(&initial_version)?
        } else {
            match manifest.find_version_from_alias(&initial_version) {
                Ok(found) => found,
                _ => manifest.find_version(&initial_version)?,
            }
        };

        debug!(target: self.get_log_target(), "Resolved to {}", candidate);

        self.set_version(candidate);

        Ok(candidate.to_owned())
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
