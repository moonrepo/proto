use crate::helpers::get_proto_home;
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use starbase_utils::{fs, toml};
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use tracing::debug;
use version_spec::UnresolvedVersionSpec;
use warpgate::{HttpOptions, Id, PluginLocator};

pub const USER_CONFIG_NAME: &str = "config.toml";

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DetectStrategy {
    #[default]
    FirstAvailable,
    PreferPrototools,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PinType {
    Global,
    Local,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UserToolConfig {
    // Partial versions allowed
    pub aliases: BTreeMap<String, UnresolvedVersionSpec>,
    pub default_version: Option<UnresolvedVersionSpec>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UserConfig {
    pub auto_clean: bool,
    pub auto_install: bool,
    pub detect_strategy: DetectStrategy,
    pub node_intercept_globals: bool,
    pub pin_latest: Option<PinType>,
    pub http: HttpOptions,
    pub plugins: BTreeMap<Id, PluginLocator>,
    pub tools: BTreeMap<Id, UserToolConfig>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl UserConfig {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> miette::Result<Self> {
        let dir = dir.as_ref();
        let path = dir.join(USER_CONFIG_NAME);

        if !path.exists() {
            return Ok(UserConfig {
                path,
                ..UserConfig::default()
            });
        }

        debug!(file = ?path, "Loading {}", USER_CONFIG_NAME);

        let contents = fs::read_file_with_lock(&path)?;
        let mut config: UserConfig = toml::from_str(&contents).into_diagnostic()?;

        let make_absolute = |file: &mut PathBuf| {
            if file.is_absolute() {
                file.to_owned()
            } else {
                dir.join(file)
            }
        };

        // Update plugin file paths to be absolute
        for locator in config.plugins.values_mut() {
            if let PluginLocator::SourceFile {
                path: ref mut source_path,
                ..
            } = locator
            {
                *source_path = make_absolute(source_path);
            }
        }

        if let Some(root_cert) = &mut config.http.root_cert {
            *root_cert = make_absolute(root_cert);
        }

        config.path = path;

        Ok(config)
    }

    #[tracing::instrument(skip_all)]
    pub fn load() -> miette::Result<Self> {
        Self::load_from(get_proto_home()?)
    }

    pub fn save(&self) -> miette::Result<()> {
        fs::write_file_with_lock(&self.path, toml::to_string_pretty(self).into_diagnostic()?)?;

        Ok(())
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            auto_clean: from_var("PROTO_AUTO_CLEAN", false),
            auto_install: from_var("PROTO_AUTO_INSTALL", false),
            detect_strategy: DetectStrategy::default(),
            http: HttpOptions::default(),
            node_intercept_globals: from_var("PROTO_NODE_INTERCEPT_GLOBALS", true),
            pin_latest: None,
            plugins: BTreeMap::default(),
            tools: BTreeMap::default(),
            path: PathBuf::new(),
        }
    }
}

fn from_var(name: &str, fallback: bool) -> bool {
    if let Ok(value) = env::var(name) {
        return value == "1" || value == "true" || value == "on";
    }

    fallback
}
