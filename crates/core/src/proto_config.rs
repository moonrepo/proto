use miette::IntoDiagnostic;
use once_cell::sync::OnceCell;
use schematic::{
    derive_enum, env, merge, Config, ConfigEnum, ConfigError, ConfigLoader, Format, PartialConfig,
    ValidateError, ValidateErrorType, ValidatorError,
};
use serde::Serialize;
use starbase_styles::color;
use starbase_utils::json::JsonValue;
use starbase_utils::toml::TomlValue;
use starbase_utils::{fs, toml};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, trace};
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

#[derive(Clone, Config, Debug, Serialize)]
#[config(allow_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoToolConfig {
    #[setting(merge = merge::merge_btreemap)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub aliases: BTreeMap<String, UnresolvedVersionSpec>,

    // Custom configuration to pass to plugins
    #[setting(merge = merge::merge_btreemap)]
    #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub config: BTreeMap<String, JsonValue>,
}

#[derive(Clone, Config, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
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

fn merge_tools(
    mut prev: BTreeMap<Id, PartialProtoToolConfig>,
    next: BTreeMap<Id, PartialProtoToolConfig>,
    context: &(),
) -> Result<Option<BTreeMap<Id, PartialProtoToolConfig>>, ConfigError> {
    for (key, value) in next {
        prev.entry(key).or_default().merge(context, value)?;
    }

    Ok(Some(prev))
}

#[derive(Clone, Config, Debug, Serialize)]
#[config(allow_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoConfig {
    #[setting(nested, merge = merge_tools)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BTreeMap<Id, ProtoToolConfig>,

    #[setting(merge = merge::merge_btreemap)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub plugins: BTreeMap<Id, PluginLocator>,

    #[setting(nested)]
    pub settings: ProtoSettingsConfig,

    #[setting(merge = merge::merge_btreemap)]
    #[serde(flatten)]
    pub versions: BTreeMap<Id, UnresolvedVersionSpec>,

    #[setting(merge = merge::merge_btreemap)]
    #[serde(flatten, skip_serializing)]
    pub unknown: BTreeMap<String, TomlValue>,
}

impl ProtoConfig {
    pub fn builtin_plugins() -> BTreeMap<Id, PluginLocator> {
        let mut config = ProtoConfig::default();
        config.inherit_builtin_plugins();
        config.plugins
    }

    pub fn inherit_builtin_plugins(&mut self) {
        if !self.plugins.contains_key("bun") {
            self.plugins.insert(
                Id::raw("bun"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/bun-plugin/releases/download/v0.6.0/bun_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("deno") {
            self.plugins.insert(
                Id::raw("deno"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/deno-plugin/releases/download/v0.6.0/deno_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("go") {
            self.plugins.insert(
                Id::raw("go"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/go-plugin/releases/download/v0.6.0/go_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("node") {
            self.plugins.insert(
                Id::raw("node"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/node-plugin/releases/download/v0.6.1/node_plugin.wasm".into()
                }
            );
        }

        for depman in ["npm", "pnpm", "yarn"] {
            if !self.plugins.contains_key(depman) {
                self.plugins.insert(
                    Id::raw(depman),
                    PluginLocator::SourceUrl {
                        url: "https://github.com/moonrepo/node-plugin/releases/download/v0.6.1/node_depman_plugin.wasm".into()
                    }
                );
            }
        }

        if !self.plugins.contains_key("python") {
            self.plugins.insert(
                Id::raw("python"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/python-plugin/releases/download/v0.4.0/python_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("rust") {
            self.plugins.insert(
                Id::raw("rust"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/rust-plugin/releases/download/v0.5.0/rust_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key(SCHEMA_PLUGIN_KEY) {
            self.plugins.insert(
                Id::raw(SCHEMA_PLUGIN_KEY),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/schema-plugin/releases/download/v0.6.0/schema_plugin.wasm".into()
                }
            );
        }
    }

    pub fn load_from<P: AsRef<Path>>(
        dir: P,
        with_lock: bool,
    ) -> miette::Result<PartialProtoConfig> {
        let dir = dir.as_ref();
        let path = dir.join(PROTO_CONFIG_NAME);

        if !path.exists() {
            return Ok(PartialProtoConfig::default());
        }

        debug!(file = ?path, "Loading {}", PROTO_CONFIG_NAME);

        let config_path = path.to_string_lossy();
        let config_content = if with_lock {
            fs::read_file_with_lock(&path)?
        } else {
            fs::read_file(&path)?
        };

        let mut config = ConfigLoader::<ProtoConfig>::new()
            .code(config_content, Format::Toml)?
            .load_partial(&())?;

        config
            .validate(&())
            .map_err(|error| ConfigError::Validator {
                config: config_path.to_string(),
                error,
                help: Some(color::muted_light("https://moonrepo.dev/docs/proto/config")),
            })?;

        // Because of serde flatten, unknown and invalid fields
        // do not trigger validation, so we need to manually handle it
        if let Some(fields) = &config.unknown {
            let mut error = ValidatorError {
                path: schematic::Path::new(vec![]),
                errors: vec![],
            };

            for (field, value) in fields {
                // Versions show up in both flattened maps...
                if config
                    .versions
                    .as_ref()
                    .is_some_and(|versions| versions.contains_key(field))
                {
                    continue;
                }

                let message = if value.is_array() || value.is_table() {
                    format!("unknown field `{field}`")
                } else {
                    match Id::new(field) {
                        Ok(_) => format!("invalid version value `{value}`"),
                        Err(e) => e.to_string(),
                    }
                };

                error.errors.push(ValidateErrorType::setting(
                    error.path.join_key(field),
                    ValidateError::new(message),
                ));
            }

            if !error.errors.is_empty() {
                return Err(ConfigError::Validator {
                    config: config_path.to_string(),
                    error,
                    help: Some(color::muted_light("https://moonrepo.dev/docs/proto/config")),
                }
                .into());
            }
        }

        // Update file paths to be absolute
        let make_absolute = |file: &mut PathBuf| {
            if file.is_absolute() {
                file.to_owned()
            } else {
                dir.join(file)
            }
        };

        if let Some(plugins) = &mut config.plugins {
            for locator in plugins.values_mut() {
                if let PluginLocator::SourceFile {
                    path: ref mut source_path,
                    ..
                } = locator
                {
                    *source_path = make_absolute(source_path);
                }
            }
        }

        if let Some(settings) = &mut config.settings {
            if let Some(http) = &mut settings.http {
                if let Some(root_cert) = &mut http.root_cert {
                    *root_cert = make_absolute(root_cert);
                }
            }
        }

        Ok(config)
    }

    pub fn save_to<P: AsRef<Path>>(dir: P, config: PartialProtoConfig) -> miette::Result<PathBuf> {
        let path = dir.as_ref().join(PROTO_CONFIG_NAME);

        fs::write_file_with_lock(&path, toml::to_string_pretty(&config).into_diagnostic()?)?;

        Ok(path)
    }

    pub fn update<P: AsRef<Path>, F: FnOnce(&mut PartialProtoConfig)>(
        dir: P,
        op: F,
    ) -> miette::Result<PathBuf> {
        let dir = dir.as_ref();
        let mut config = Self::load_from(dir, true)?;

        op(&mut config);

        Self::save_to(dir, config)
    }
}

#[derive(Debug, Serialize)]
pub struct ProtoConfigFile {
    pub exists: bool,
    pub global: bool,
    pub path: PathBuf,
    pub config: PartialProtoConfig,
}

#[derive(Debug)]
pub struct ProtoConfigManager {
    // Paths are sorted from current working directory,
    // up until the root or user directory, whichever is first.
    // The special `~/.proto/.prototools` config is always
    // loaded last, and is the last entry in the list.
    // For directories without a config, we still insert
    // an empty entry. This helps with traversal logic.
    pub files: Vec<ProtoConfigFile>,

    all_config: Arc<OnceCell<ProtoConfig>>,
    all_config_no_global: Arc<OnceCell<ProtoConfig>>,
    cwd_config: Arc<OnceCell<ProtoConfig>>,
}

impl ProtoConfigManager {
    pub fn load(start_dir: impl AsRef<Path>, end_dir: Option<&Path>) -> miette::Result<Self> {
        trace!("Traversing upwards and loading {} files", PROTO_CONFIG_NAME);

        let mut current_dir = Some(start_dir.as_ref());
        let mut files = vec![];

        while let Some(dir) = current_dir {
            let path = dir.join(PROTO_CONFIG_NAME);

            files.push(ProtoConfigFile {
                exists: path.exists(),
                global: false,
                path,
                config: ProtoConfig::load_from(dir, false)?,
            });

            if end_dir.is_some_and(|end| end == dir) {
                break;
            }

            current_dir = dir.parent();
        }

        Ok(Self {
            files,
            all_config: Arc::new(OnceCell::new()),
            all_config_no_global: Arc::new(OnceCell::new()),
            cwd_config: Arc::new(OnceCell::new()),
        })
    }

    pub fn get_local_config(&self) -> miette::Result<&ProtoConfig> {
        self.cwd_config.get_or_try_init(|| {
            debug!("Merging local config");
            self.merge_configs(vec![&self.files[0]])
        })
    }

    pub fn get_merged_config(&self) -> miette::Result<&ProtoConfig> {
        self.all_config.get_or_try_init(|| {
            debug!("Merging loaded configs");
            self.merge_configs(self.files.iter().collect())
        })
    }

    pub fn get_merged_config_without_global(&self) -> miette::Result<&ProtoConfig> {
        self.all_config_no_global.get_or_try_init(|| {
            debug!("Merging loaded configs without global");
            self.merge_configs(self.files.iter().filter(|file| !file.global).collect())
        })
    }

    fn merge_configs(&self, files: Vec<&ProtoConfigFile>) -> miette::Result<ProtoConfig> {
        let mut partial = PartialProtoConfig::default();
        let mut count = 0;
        let context = &();

        for file in files.iter().rev() {
            if file.exists {
                partial.merge(context, file.config.to_owned())?;
                count += 1;
            }
        }

        let mut config = ProtoConfig::from_partial(partial.finalize(context)?);
        config.inherit_builtin_plugins();

        debug!("Merged {} configs", count);

        Ok(config)
    }
}
