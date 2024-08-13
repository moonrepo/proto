use crate::helpers::ENV_VAR_SUB;
use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use rustc_hash::FxHashMap;
use schematic::{
    derive_enum, env, merge, Config, ConfigEnum, ConfigError, ConfigLoader, DefaultValueResult,
    Format, MergeError, MergeResult, PartialConfig, Path as ErrorPath, ValidateError,
    ValidateResult, ValidatorError,
};
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use starbase_utils::json::JsonValue;
use starbase_utils::toml::TomlValue;
use starbase_utils::{fs, toml};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, instrument};
use version_spec::*;
use warpgate::{HttpOptions, Id, PluginLocator, UrlLocator};

pub const PROTO_CONFIG_NAME: &str = ".prototools";
pub const SCHEMA_PLUGIN_KEY: &str = "internal-schema";

fn merge_tools(
    mut prev: BTreeMap<Id, PartialProtoToolConfig>,
    next: BTreeMap<Id, PartialProtoToolConfig>,
    context: &(),
) -> MergeResult<BTreeMap<Id, PartialProtoToolConfig>> {
    for (key, value) in next {
        prev.entry(key)
            .or_default()
            .merge(context, value)
            .map_err(MergeError::new)?;
    }

    Ok(Some(prev))
}

fn merge_fxhashmap<K, V, C>(
    mut prev: FxHashMap<K, V>,
    next: FxHashMap<K, V>,
    _context: &C,
) -> MergeResult<FxHashMap<K, V>>
where
    K: Eq + Hash,
{
    for (key, value) in next {
        prev.insert(key, value);
    }

    Ok(Some(prev))
}

fn merge_indexmap<K, V>(
    mut prev: IndexMap<K, V>,
    next: IndexMap<K, V>,
    _context: &(),
) -> MergeResult<IndexMap<K, V>>
where
    K: Eq + Hash,
{
    for (key, value) in next {
        prev.insert(key, value);
    }

    Ok(Some(prev))
}

fn validate_reserved_words(
    value: &BTreeMap<Id, PluginLocator>,
    _partial: &PartialProtoConfig,
    _context: &(),
    _finalize: bool,
) -> ValidateResult {
    if value.contains_key("proto") {
        return Err(ValidateError::new(
            "proto is a reserved keyword, cannot use as a plugin identifier",
        ));
    }

    Ok(())
}

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum ConfigMode {
        Global,
        Local,
        Upwards,
        #[default]
        #[serde(alias = "all")]
        UpwardsGlobal,
    }
);

impl ConfigMode {
    pub fn includes_global(&self) -> bool {
        matches!(self, Self::Global | Self::UpwardsGlobal)
    }

    pub fn only_local(&self) -> bool {
        matches!(self, Self::Local)
    }
}

derive_enum!(
    #[derive(ConfigEnum, Default)]
    pub enum DetectStrategy {
        #[default]
        FirstAvailable,
        PreferPrototools,
        OnlyPrototools,
    }
);

derive_enum!(
    #[derive(ConfigEnum)]
    pub enum PinType {
        Global,
        Local,
    }
);

#[derive(Clone, Config, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum EnvVar {
    State(bool),
    Value(String),
}

impl EnvVar {
    pub fn to_value(&self) -> Option<String> {
        match self {
            Self::State(state) => state.then(|| "true".to_owned()),
            Self::Value(value) => Some(value.to_owned()),
        }
    }
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

#[derive(Clone, Config, Debug, Serialize)]
#[config(allow_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoToolConfig {
    #[setting(merge = merge::merge_btreemap)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub aliases: BTreeMap<String, UnresolvedVersionSpec>,

    #[setting(nested, merge = merge_indexmap)]
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub env: IndexMap<String, EnvVar>,

    // Custom configuration to pass to plugins
    #[setting(merge = merge_fxhashmap)]
    #[serde(flatten, skip_serializing_if = "FxHashMap::is_empty")]
    pub config: FxHashMap<String, JsonValue>,
}

#[derive(Clone, Config, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoSettingsConfig {
    #[setting(env = "PROTO_AUTO_CLEAN", parse_env = env::parse_bool)]
    pub auto_clean: bool,

    #[setting(env = "PROTO_AUTO_INSTALL", parse_env = env::parse_bool)]
    pub auto_install: bool,

    #[setting(default = default_builtin_plugins)]
    pub builtin_plugins: BuiltinPlugins,

    #[setting(env = "PROTO_DETECT_STRATEGY")]
    pub detect_strategy: DetectStrategy,

    pub http: HttpOptions,

    #[setting(env = "PROTO_PIN_LATEST")]
    pub pin_latest: Option<PinType>,

    #[setting(default = true)]
    pub telemetry: bool,
}

#[derive(Clone, Config, Debug, Serialize)]
#[config(allow_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoConfig {
    #[setting(nested, merge = merge_indexmap)]
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub env: IndexMap<String, EnvVar>,

    #[setting(nested, merge = merge_tools)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BTreeMap<Id, ProtoToolConfig>,

    #[setting(merge = merge::merge_btreemap, validate = validate_reserved_words)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub plugins: BTreeMap<Id, PluginLocator>,

    #[setting(nested)]
    pub settings: ProtoSettingsConfig,

    #[setting(merge = merge::merge_btreemap)]
    #[serde(flatten)]
    pub versions: BTreeMap<Id, UnresolvedVersionSpec>,

    #[setting(merge = merge_fxhashmap)]
    #[serde(flatten, skip_serializing)]
    pub unknown: FxHashMap<String, TomlValue>,
}

impl ProtoConfig {
    pub fn builtin_plugins(&self) -> BTreeMap<Id, PluginLocator> {
        let mut config = ProtoConfig::default();

        // Inherit this setting in case builtins have been disabled
        config.settings.builtin_plugins = self.settings.builtin_plugins.clone();

        // Then inherit all the available builtins
        config.inherit_builtin_plugins();

        config.plugins
    }

    pub fn inherit_builtin_plugins(&mut self) {
        let is_allowed = |id: &str| match &self.settings.builtin_plugins {
            BuiltinPlugins::Enabled(state) => *state,
            BuiltinPlugins::Allowed(list) => list.iter().any(|aid| aid == id),
        };

        if !self.plugins.contains_key("bun") && is_allowed("bun") {
            self.plugins.insert(
                Id::raw("bun"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/tools/releases/download/bun_tool-v0.12.3/bun_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("deno") && is_allowed("deno") {
            self.plugins.insert(
                Id::raw("deno"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/tools/releases/download/deno_tool-v0.11.4/deno_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("go") && is_allowed("go") {
            self.plugins.insert(
                Id::raw("go"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/tools/releases/download/go_tool-v0.12.0/go_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("node") && is_allowed("node") {
            self.plugins.insert(
                Id::raw("node"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/tools/releases/download/node_tool-v0.11.8/node_tool.wasm".into()
                }))
            );
        }

        for depman in ["npm", "pnpm", "yarn"] {
            if !self.plugins.contains_key(depman) && is_allowed(depman) {
                self.plugins.insert(
                    Id::raw(depman),
                    PluginLocator::Url(Box::new(UrlLocator {
                        url: "https://github.com/moonrepo/tools/releases/download/node_depman_tool-v0.12.0/node_depman_tool.wasm".into()
                    }))
                );
            }
        }

        if !self.plugins.contains_key("python") && is_allowed("python") {
            self.plugins.insert(
                Id::raw("python"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/tools/releases/download/python_tool-v0.10.5/python_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("rust") && is_allowed("rust") {
            self.plugins.insert(
                Id::raw("rust"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/tools/releases/download/rust_tool-v0.10.6/rust_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key(SCHEMA_PLUGIN_KEY) {
            self.plugins.insert(
                Id::raw(SCHEMA_PLUGIN_KEY),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/tools/releases/download/schema_tool-v0.14.1/schema_tool.wasm".into()
                }))
            );
        }
    }

    pub fn load_from<P: AsRef<Path>>(
        dir: P,
        with_lock: bool,
    ) -> miette::Result<PartialProtoConfig> {
        let dir = dir.as_ref();

        Self::load(
            if dir.ends_with(PROTO_CONFIG_NAME) {
                dir.to_path_buf()
            } else {
                dir.join(PROTO_CONFIG_NAME)
            },
            with_lock,
        )
    }

    #[instrument(name = "load_config")]
    pub fn load<P: AsRef<Path> + Debug>(
        path: P,
        with_lock: bool,
    ) -> miette::Result<PartialProtoConfig> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(PartialProtoConfig::default());
        }

        debug!(file = ?path, "Loading {}", PROTO_CONFIG_NAME);

        let config_path = path.to_string_lossy();
        let config_content = if with_lock {
            fs::read_file_with_lock(path)?
        } else {
            fs::read_file(path)?
        };

        let mut config = ConfigLoader::<ProtoConfig>::new()
            .code(config_content, Format::Toml)?
            .load_partial(&())?;

        config.validate(&(), true).map_err(|error| match error {
            ConfigError::Validator { error, .. } => ConfigError::Validator {
                location: config_path.to_string(),
                error,
                help: Some(color::muted_light("https://moonrepo.dev/docs/proto/config")),
            },
            _ => error,
        })?;

        // Because of serde flatten, unknown and invalid fields
        // do not trigger validation, so we need to manually handle it
        if let Some(fields) = &config.unknown {
            let mut error = ValidatorError { errors: vec![] };

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

                error.errors.push(ValidateError::with_path(
                    message,
                    ErrorPath::default().join_key(field),
                ));
            }

            if !error.errors.is_empty() {
                return Err(ConfigError::Validator {
                    location: config_path.to_string(),
                    error: Box::new(error),
                    help: Some(color::muted_light("https://moonrepo.dev/docs/proto/config")),
                }
                .into());
            }
        }

        // Update file paths to be absolute
        let make_absolute = |file: &PathBuf| {
            if file.is_absolute() {
                file.to_owned()
            } else if let Some(dir) = path.parent() {
                dir.join(file)
            } else {
                PathBuf::from("/").join(file)
            }
        };

        if let Some(plugins) = &mut config.plugins {
            for locator in plugins.values_mut() {
                if let PluginLocator::File(ref mut inner) = locator {
                    inner.path = Some(make_absolute(&inner.get_unresolved_path()));
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

    #[instrument(name = "save_config", skip(config))]
    pub fn save_to<P: AsRef<Path> + Debug>(
        dir: P,
        config: PartialProtoConfig,
    ) -> miette::Result<PathBuf> {
        let path = dir.as_ref();
        let file = if path.ends_with(PROTO_CONFIG_NAME) {
            path.to_path_buf()
        } else {
            path.join(PROTO_CONFIG_NAME)
        };

        fs::write_file_with_lock(&file, toml::format(&config, true)?)?;

        Ok(file)
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

    // We don't use a `BTreeMap` for env vars, so that variable interpolation
    // and order of declaration can work correctly!
    pub fn get_env_vars(
        &self,
        filter_id: Option<&Id>,
    ) -> miette::Result<IndexMap<String, Option<String>>> {
        let mut base_vars = IndexMap::new();
        base_vars.extend(self.env.iter());

        if let Some(id) = filter_id {
            if let Some(tool_config) = self.tools.get(id) {
                base_vars.extend(tool_config.env.iter())
            }
        }

        let mut vars = IndexMap::<String, Option<String>>::new();

        for (key, value) in base_vars {
            let key_exists = std::env::var(key).is_ok_and(|v| !v.is_empty());
            let value = value.to_value();

            // Don't override parent inherited vars
            if key_exists && value.is_some() {
                continue;
            }

            // Interpolate nested vars
            let value = value.map(|val| {
                ENV_VAR_SUB
                    .replace_all(&val, |cap: &regex::Captures| {
                        let name = cap.get(1).unwrap().as_str();

                        if let Ok(existing) = std::env::var(name) {
                            existing
                        } else if let Some(Some(existing)) = vars.get(name) {
                            existing.to_owned()
                        } else {
                            String::new()
                        }
                    })
                    .to_string()
            });

            vars.insert(key.to_owned(), value);
        }

        Ok(vars)
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
    global_config: Arc<OnceCell<ProtoConfig>>,
    local_config: Arc<OnceCell<ProtoConfig>>,
}

impl ProtoConfigManager {
    pub fn load(
        start_dir: impl AsRef<Path>,
        end_dir: Option<&Path>,
        env_mode: Option<&String>,
    ) -> miette::Result<Self> {
        let mut current_dir = Some(start_dir.as_ref());
        let mut files = vec![];

        while let Some(dir) = current_dir {
            if let Some(env) = env_mode {
                let env_path = dir.join(format!("{}.{env}", PROTO_CONFIG_NAME));

                files.push(ProtoConfigFile {
                    config: ProtoConfig::load(&env_path, false)?,
                    exists: env_path.exists(),
                    global: false,
                    path: env_path,
                });
            }

            let path = dir.join(PROTO_CONFIG_NAME);

            files.push(ProtoConfigFile {
                config: ProtoConfig::load(&path, false)?,
                exists: path.exists(),
                global: false,
                path,
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
            global_config: Arc::new(OnceCell::new()),
            local_config: Arc::new(OnceCell::new()),
        })
    }

    pub fn get_global_config(&self) -> miette::Result<&ProtoConfig> {
        self.global_config.get_or_try_init(|| {
            debug!("Loading global config only");

            self.merge_configs(self.files.iter().filter(|file| file.global).collect())
        })
    }

    pub fn get_local_config(&self, cwd: &Path) -> miette::Result<&ProtoConfig> {
        self.local_config.get_or_try_init(|| {
            debug!("Loading local config only");

            self.merge_configs(
                self.files
                    .iter()
                    .filter(|file| file.path.parent().is_some_and(|dir| dir == cwd))
                    .collect(),
            )
        })
    }

    pub fn get_merged_config(&self) -> miette::Result<&ProtoConfig> {
        self.all_config.get_or_try_init(|| {
            debug!("Merging loaded configs with global");

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
