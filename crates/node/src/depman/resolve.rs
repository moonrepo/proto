use crate::depman::{NodeDependencyManager, NodeDependencyManagerType};
use crate::platform::PackageJson;
use crate::NodeLanguage;
use log::debug;
use proto_core::{
    async_trait, detect_version_from_environment, is_offline, is_semantic_version,
    load_versions_manifest, parse_version, remove_v_prefix, Describable, Proto, ProtoError,
    Resolvable, VersionManifest, VersionManifestEntry,
};
use rustc_hash::FxHashMap;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct NDMVersionDistSignature {
    pub keyid: String,
    pub sig: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NDMVersionDist {
    pub integrity: String,
    pub shasum: String,
    #[serde(rename = "npm-signature")]
    pub signature: Option<String>,
    pub signatures: Vec<NDMVersionDistSignature>,
    pub tarball: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NDMVersion {
    // dist: NDMVersionDist,
    version: String, // No v prefix
}

#[derive(Deserialize)]
struct NDMManifest {
    #[serde(rename = "dist-tags")]
    dist_tags: FxHashMap<String, String>,
    versions: FxHashMap<String, NDMVersion>,
}

#[async_trait]
impl Resolvable<'_> for NodeDependencyManager {
    fn get_resolved_version(&self) -> &str {
        match self.version.as_ref() {
            Some(version) => version,
            None => "latest",
        }
    }

    async fn load_manifest(&self) -> Result<VersionManifest, ProtoError> {
        let mut versions = BTreeMap::new();
        let response: NDMManifest =
            load_versions_manifest(format!("https://registry.npmjs.org/{}/", self.package_name))
                .await?;

        for item in response.versions.values() {
            versions.insert(
                item.version.clone(),
                VersionManifestEntry {
                    alias: None,
                    version: item.version.clone(),
                },
            );
        }

        Ok(VersionManifest {
            // Aliases map to dist tags
            aliases: BTreeMap::from_iter(response.dist_tags),
            versions,
        })
    }

    async fn resolve_version(&mut self, initial_version: &str) -> Result<String, ProtoError> {
        if let Some(version) = &self.version {
            return Ok(version.to_owned());
        }

        let mut initial_version = remove_v_prefix(initial_version);

        match &self.type_of {
            // When the alias "bundled" is provided, we should install the npm
            // version that comes bundled with the default Node.js version.
            NodeDependencyManagerType::Npm => {
                if initial_version == "bundled" {
                    let node_tool = Box::new(NodeLanguage::new(Proto::new()?));

                    if let Ok(node_version) =
                        detect_version_from_environment(&node_tool, None).await
                    {
                        let npm_package_path = node_tool
                            .base_dir
                            .join(node_version)
                            .join(if cfg!(windows) {
                                "node_modules"
                            } else {
                                "lib/node_modules"
                            })
                            .join("npm/package.json");

                        if let Ok(npm_package) = PackageJson::load(&npm_package_path) {
                            if let Some(npm_version) = npm_package.version {
                                initial_version = npm_version;
                            }
                        }
                    }
                }
            }

            NodeDependencyManagerType::Pnpm => {}

            // Yarn is installed through npm, but only v1 exists in the npm registry,
            // even if a consumer is using Yarn 2/3. https://www.npmjs.com/package/yarn
            // Yarn >= 2 works differently than normal packages, as their runtime code
            // is stored *within* the repository, and the v1 package detects it.
            // Because of this, we need to always install the v1 package!
            NodeDependencyManagerType::Yarn => {
                if !initial_version.starts_with('1') {
                    debug!(
                        target: self.get_log_target(),
                        "Found Yarn v2+, installing latest v1 from registry for compatibility"
                    );

                    initial_version = if is_offline() {
                        "1.22.19".to_owned() // This may change upstream!
                    } else {
                        "latest".to_owned()
                    };
                }
            }
        };

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
        let version = parse_version(manifest.find_version(&initial_version)?)?.to_string();

        debug!(target: self.get_log_target(), "Resolved to {}", version);

        self.set_version(&version);

        // Extract dist information for use in downloading and verifying
        // self.dist = Some(
        //     manifest
        //         .versions
        //         .get(candidate.unwrap())
        //         .unwrap()
        //         .dist
        //         .clone(),
        // );

        Ok(version)
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
