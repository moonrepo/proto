use proto_core::flow::resolve::ProtoResolveError;
use proto_core::{
    ProtoConfig, ProtoToolConfig, Tool, ToolSpec, UnresolvedVersionSpec, VersionSpec,
};
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

#[derive(Debug)]
pub struct ToolRecord {
    pub tool: Tool,
    pub config: ProtoToolConfig,
    pub detected_source: Option<PathBuf>,
    pub detected_version: Option<ToolSpec>,
    pub installed_versions: Vec<VersionSpec>,
    pub local_aliases: BTreeMap<String, ToolSpec>,
    pub remote_aliases: BTreeMap<String, ToolSpec>,
    pub remote_versions: Vec<VersionSpec>,
}

impl ToolRecord {
    pub fn new(tool: Tool) -> Self {
        let mut versions = tool
            .inventory
            .manifest
            .installed_versions
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        versions.sort();

        Self {
            tool,
            config: ProtoToolConfig::default(),
            detected_source: None,
            detected_version: None,
            local_aliases: BTreeMap::default(),
            remote_aliases: BTreeMap::default(),
            installed_versions: versions,
            remote_versions: vec![],
        }
    }

    pub async fn detect_version_and_source(&mut self) {
        if let Ok(config_version) = self.tool.detect_version().await {
            self.detected_version = Some(config_version);
            self.detected_source =
                std::env::var(format!("{}_DETECTED_FROM", self.tool.get_env_var_prefix()))
                    .ok()
                    .map(PathBuf::from);
        }
    }

    pub fn inherit_from_local(&mut self, config: &ProtoConfig) {
        if let Some(tool_config) = config.tools.get(self.get_id()).map(|c| c.to_owned()) {
            self.local_aliases.extend(tool_config.aliases.clone());
            self.config = tool_config;
        }
    }

    pub async fn inherit_from_remote(&mut self) -> Result<(), ProtoResolveError> {
        let version_resolver = self
            .tool
            .load_version_resolver(&UnresolvedVersionSpec::default())
            .await?;

        self.remote_aliases.extend(
            version_resolver
                .aliases
                .into_iter()
                .map(|(k, v)| (k, ToolSpec::new(v))),
        );
        self.remote_versions.extend(version_resolver.versions);
        self.remote_versions.sort();

        Ok(())
    }
}

impl Deref for ToolRecord {
    type Target = Tool;

    fn deref(&self) -> &Self::Target {
        &self.tool
    }
}

impl DerefMut for ToolRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tool
    }
}
