use indexmap::IndexMap;
use proto_core::{ConfigMode, ProtoConfig, ToolContext, VersionSpec};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use system_env::{SystemArch, SystemOS};

#[derive(Serialize)]
pub struct ConfigResource<'a> {
    pub working_dir: PathBuf,
    pub config_mode: ConfigMode,
    pub config_files: Vec<&'a PathBuf>,
    pub config: &'a ProtoConfig,
}

#[derive(Serialize)]
pub struct EnvResource<'a> {
    pub working_dir: PathBuf,
    pub store_dir: PathBuf,
    pub env_mode: Option<String>,
    pub env_files: Vec<&'a PathBuf>,
    pub env_vars: IndexMap<String, Option<String>>,
    pub proto_version: String,
    pub system_arch: SystemArch,
    pub system_os: SystemOS,
}

#[derive(Serialize)]
pub struct ToolsResource {
    pub tools: BTreeMap<ToolContext, ToolResourceEntry>,
}

#[derive(Serialize)]
pub struct ToolResourceEntry {
    pub tool_dir: PathBuf,
    pub installed_versions: Vec<VersionSpec>,
}
