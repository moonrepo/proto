use super::{DetectStrategy, PinLocation, merge_iter};
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use schematic::{Config, DefaultValueResult, RegexSetting, env, merge};
use serde::{Deserialize, Serialize};
use system_env::{SystemOS, SystemPackageManager};
use warpgate::{HttpOptions, RegistryConfig};

// `[settings.build]`
#[derive(Clone, Config, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoBuildConfig {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[setting(env = "PROTO_BUILD_EXCLUDE_PACKAGES", parse_env = env::split_comma)]
    pub exclude_packages: Vec<String>,

    #[setting(
        default = true,
        env = "PROTO_BUILD_INSTALL_SYSTEM_PACKAGES",
        parse_env = env::parse_bool,
    )]
    pub install_system_packages: bool,

    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub system_package_manager: FxHashMap<SystemOS, Option<SystemPackageManager>>,

    #[setting(env = "PROTO_BUILD_WRITE_LOG_FILE", parse_env = env::parse_bool)]
    pub write_log_file: bool,
}

// `[settings.offline]`
#[derive(Clone, Config, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoOfflineConfig {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[setting(env = "PROTO_OFFLINE_HOSTS", parse_env = env::split_comma)]
    pub custom_hosts: Vec<String>,

    #[setting(env = "PROTO_OFFLINE_OVERRIDE_HOSTS", parse_env = env::parse_bool)]
    pub override_default_hosts: bool,

    #[setting(default = 750, env = "PROTO_OFFLINE_TIMEOUT")]
    pub timeout: u64,
}

#[derive(Clone, Config, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BuiltinPlugins {
    Enabled(bool),
    Allowed(Vec<String>),
}

fn default_builtin_plugins(_context: &()) -> DefaultValueResult<BuiltinPlugins> {
    Ok(Some(BuiltinPlugins::Enabled(true)))
}

// `[settings]`
#[derive(Clone, Config, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoSettingsConfig {
    #[setting(env = "PROTO_AUTO_CLEAN", parse_env = env::parse_bool)]
    pub auto_clean: bool,

    #[setting(env = "PROTO_AUTO_INSTALL", parse_env = env::parse_bool)]
    pub auto_install: bool,

    #[setting(nested)]
    pub build: ProtoBuildConfig,

    #[setting(default = default_builtin_plugins)]
    pub builtin_plugins: BuiltinPlugins,

    #[setting(env = "PROTO_CACHE_DURATION")]
    pub cache_duration: Option<u64>,

    #[setting(env = "PROTO_DETECT_STRATEGY")]
    pub detect_strategy: DetectStrategy,

    pub http: HttpOptions,

    #[serde(alias = "unstable-lockfile")]
    pub lockfile: bool,

    #[setting(nested)]
    pub offline: ProtoOfflineConfig,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[setting(env = "PROTO_PIN_LATEST")]
    pub pin_latest: Option<PinLocation>,

    #[serde(alias = "unstable-registries", skip_serializing_if = "Vec::is_empty")]
    #[setting(merge = merge::append_vec)]
    pub registries: Vec<RegistryConfig>,

    #[setting(default = true, env = "PROTO_TELEMETRY", parse_env = env::parse_bool)]
    pub telemetry: bool,

    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    #[setting(merge = merge_iter)]
    pub url_rewrites: IndexMap<RegexSetting, String>,
}
