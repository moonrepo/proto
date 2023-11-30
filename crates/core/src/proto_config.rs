use schematic::{derive_enum, env, Config, ConfigEnum};
use starbase_utils::toml::TomlValue;
use std::collections::BTreeMap;
use version_spec::*;
use warpgate::{HttpOptions, Id, PluginLocator};

pub const PROTO_CONFIG_NAME: &str = ".prototools";
pub const SCHEMA_PLUGIN_KEY: &str = "internal-schema";

derive_enum!(
    #[derive(ConfigEnum, Default)]
    pub enum DetectStrategy {
        #[default]
        FirstAvailable,
        PreferPrototools,
    }
);

derive_enum!(
    #[derive(ConfigEnum)]
    pub enum PinType {
        Global,
        Local,
    }
);

#[derive(Config)]
#[config(allow_unknown_fields, rename_all = "kebab-case")]
pub struct ProtoToolConfig {
    pub aliases: BTreeMap<String, UnresolvedVersionSpec>,

    // Custom configuration to pass to plugins
    #[setting(flatten)]
    pub config: BTreeMap<String, TomlValue>,
}

#[derive(Config)]
#[config(rename_all = "kebab-case")]
pub struct ProtoSettingsConfig {
    #[setting(env = "PROTO_AUTO_CLEAN", parse_env = env::parse_bool)]
    pub auto_clean: bool,

    #[setting(env = "PROTO_AUTO_INSTALL", parse_env = env::parse_bool)]
    pub auto_install: bool,

    #[setting(env = "PROTO_DETECT_STRATEGY")]
    pub detect_strategy: DetectStrategy,

    #[setting(env = "PROTO_PIN_LATEST")]
    pub pin_latest: Option<PinType>,

    pub http: HttpOptions,
}

#[derive(Config)]
#[config(allow_unknown_fields, rename_all = "kebab-case")]
pub struct ProtoConfig {
    pub plugins: BTreeMap<Id, PluginLocator>,

    #[setting(nested)]
    pub settings: ProtoSettingsConfig,

    #[setting(nested)]
    pub tools: BTreeMap<Id, ProtoToolConfig>,

    #[setting(flatten)]
    pub versions: BTreeMap<Id, UnresolvedVersionSpec>,
}
