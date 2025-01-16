use core::ops::{Deref, DerefMut};
use proto_core::{
    detect_version, ProtoConfig, ProtoToolConfig, Tool, UnresolvedVersionSpec, VersionSpec,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub struct ToolRecord {
    pub tool: Tool,
    pub config: ProtoToolConfig,
    pub detected_source: Option<PathBuf>,
    pub detected_version: Option<UnresolvedVersionSpec>,
    pub installed_versions: Vec<VersionSpec>,
    pub local_aliases: BTreeMap<String, UnresolvedVersionSpec>,
    pub remote_aliases: BTreeMap<String, UnresolvedVersionSpec>,
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

    pub async fn detect_version(&mut self) {
        if let Ok(config_version) = detect_version(&self.tool, None).await {
            self.detected_version = Some(config_version);
            self.detected_source =
                std::env::var(format!("{}_DETECTED_FROM", self.tool.get_env_var_prefix()))
                    .ok()
                    .map(PathBuf::from);
        }
    }

    pub fn inherit_from_local(&mut self, config: &ProtoConfig) {
        if let Some(tool_config) = config.tools.get(&self.id).map(|c| c.to_owned()) {
            self.local_aliases.extend(tool_config.aliases.clone());
            self.config = tool_config;
        }
    }

    pub async fn inherit_from_remote(&mut self) -> miette::Result<()> {
        let version_resolver = self
            .tool
            .load_version_resolver(&UnresolvedVersionSpec::default())
            .await?;

        self.remote_aliases.extend(version_resolver.aliases);
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
