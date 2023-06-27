use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// HOST FUNCTIONS

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EmptyInput {}

// PLUGIN API

#[derive(Debug, Default, Deserialize, Serialize)]
pub enum PluginType {
    #[default]
    Language,
    DependencyManager,
    CLI,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ToolMetadataInput {
    pub id: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ToolMetadata {
    pub name: String,
    pub type_of: PluginType,
}

// Common

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EnvironmentInput {
    pub arch: String,
    pub os: String,
    pub version: String,
}

// Detector

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct DetectVersionFiles {
    pub files: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ParseVersionInput {
    pub content: String,
    pub file: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ParseVersion {
    pub version: Option<String>,
}

// Downloader, Installer, Verifier

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct InstallParams {
    pub archive_prefix: Option<String>,
    pub bin_path: Option<String>,
    pub checksum_file: Option<String>,
    pub checksum_url: Option<String>,
    pub download_file: Option<String>,
    pub download_url: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct UnpackArchiveInput {
    pub download_path: PathBuf,
    pub install_dir: PathBuf,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct VerifyChecksumInput {
    pub checksum_path: PathBuf,
    pub download_path: PathBuf,
    pub version: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct VerifyChecksum {
    pub verified: bool,
}

// Executor

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ExecuteInput {
    pub env: EnvironmentInput,
    pub tool_dir: PathBuf,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ExecuteParams {
    pub bin_path: Option<String>,
    pub globals_dir: Vec<String>,
}

// Shimmer

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ShimConfig {
    pub bin_path: String,
    pub parent_bin: Option<String>,
    pub before_args: Option<String>,
    pub after_args: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ShimParams {
    pub primary: Option<ShimConfig>,
    pub global_shims: HashMap<String, String>,
    pub local_shims: HashMap<String, ShimConfig>,
}
