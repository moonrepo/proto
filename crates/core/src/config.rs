use crate::config_error::ProtoConfigError;
use crate::helpers::ENV_VAR_SUB;
use crate::tool_spec::{Backend, ToolSpec};
use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use rustc_hash::FxHashMap;
use schematic::{
    Config, ConfigEnum, ConfigError, ConfigLoader, DefaultValueResult, Format, MergeError,
    MergeResult, PartialConfig, Path as ErrorPath, ValidateError, ValidateResult, ValidatorError,
    derive_enum, env, merge,
};
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use starbase_utils::fs::FsError;
use starbase_utils::json::JsonValue;
use starbase_utils::toml::TomlValue;
use starbase_utils::{fs, toml};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use system_env::{SystemOS, SystemPackageManager};
use tracing::{debug, instrument};
use warpgate::{HttpOptions, Id, PluginLocator, UrlLocator};

pub const PROTO_CONFIG_NAME: &str = ".prototools";
pub const SCHEMA_PLUGIN_KEY: &str = "internal-schema";
pub const PROTO_PLUGIN_KEY: &str = "proto";
pub const ENV_FILE_KEY: &str = "file";

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
    if value.contains_key(PROTO_PLUGIN_KEY) {
        return Err(ValidateError::new(
            "proto is a reserved keyword, cannot use as a plugin identifier",
        ));
    }

    Ok(())
}

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    #[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
    pub enum ConfigMode {
        Global,
        Local,
        Upwards,
        #[default]
        #[serde(alias = "all")]
        #[cfg_attr(feature = "clap", value(alias("all")))]
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
    #[derive(Copy, ConfigEnum, Default)]
    #[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
    pub enum PinLocation {
        #[serde(alias = "store")]
        #[cfg_attr(feature = "clap", value(alias("store")))]
        Global,
        #[default]
        #[serde(alias = "cwd")]
        #[cfg_attr(feature = "clap", value(alias("cwd")))]
        Local,
        #[serde(alias = "home")]
        #[cfg_attr(feature = "clap", value(alias("home")))]
        User,
    }
);

#[derive(Clone, Debug, PartialEq)]
pub struct EnvFile {
    pub path: PathBuf,
    pub weight: usize,
}

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
#[serde(rename_all = "kebab-case")]
pub struct ProtoBuildConfig {
    pub exclude_packages: Vec<String>,

    #[setting(default = true)]
    pub install_system_packages: bool,

    pub system_package_manager: FxHashMap<SystemOS, Option<SystemPackageManager>>,

    pub write_log_file: bool,
}

#[derive(Clone, Config, Debug, Serialize)]
#[config(allow_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoToolConfig {
    #[setting(merge = merge::merge_btreemap)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub aliases: BTreeMap<String, ToolSpec>,

    pub backend: Option<Backend>,

    #[setting(nested, merge = merge_indexmap)]
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub env: IndexMap<String, EnvVar>,

    // Custom configuration to pass to plugins
    #[setting(merge = merge_fxhashmap)]
    #[serde(flatten, skip_serializing_if = "FxHashMap::is_empty")]
    pub config: FxHashMap<String, JsonValue>,

    #[setting(exclude, merge = merge::append_vec)]
    #[serde(skip)]
    _env_files: Vec<EnvFile>,
}

#[derive(Clone, Config, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoOfflineConfig {
    pub custom_hosts: Vec<String>,

    pub override_default_hosts: bool,

    #[setting(default = 750)]
    pub timeout: u64,
}

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

    #[setting(env = "PROTO_DETECT_STRATEGY")]
    pub detect_strategy: DetectStrategy,

    pub http: HttpOptions,

    #[setting(nested)]
    pub offline: ProtoOfflineConfig,

    #[setting(env = "PROTO_PIN_LATEST")]
    pub pin_latest: Option<PinLocation>,

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
    pub versions: BTreeMap<Id, ToolSpec>,

    #[setting(merge = merge_fxhashmap)]
    #[serde(flatten, skip_serializing)]
    pub unknown: FxHashMap<String, TomlValue>,

    #[setting(exclude, merge = merge::append_vec)]
    #[serde(skip)]
    _env_files: Vec<EnvFile>,
}

impl ProtoConfig {
    pub fn setup_env_vars(&self) {
        use std::env;

        if env::var("PROTO_OFFLINE_OVERRIDE_HOSTS").is_err()
            && self.settings.offline.override_default_hosts
        {
            unsafe { env::set_var("PROTO_OFFLINE_OVERRIDE_HOSTS", "true") };
        }

        if env::var("PROTO_OFFLINE_HOSTS").is_err()
            && !self.settings.offline.custom_hosts.is_empty()
        {
            unsafe {
                env::set_var(
                    "PROTO_OFFLINE_HOSTS",
                    self.settings.offline.custom_hosts.join(","),
                )
            };
        }

        if env::var("PROTO_OFFLINE_TIMEOUT").is_err() {
            unsafe {
                env::set_var(
                    "PROTO_OFFLINE_TIMEOUT",
                    self.settings.offline.timeout.to_string(),
                )
            };
        }
    }

    pub fn builtin_plugins(&self) -> BTreeMap<Id, PluginLocator> {
        let mut config = ProtoConfig::default();

        // Inherit this setting in case builtins have been disabled
        config.settings.builtin_plugins = self.settings.builtin_plugins.clone();

        // Then inherit all the available builtins
        config.inherit_builtin_plugins();

        config.plugins
    }

    pub fn builtin_proto_plugin(&self) -> PluginLocator {
        PluginLocator::Url(Box::new(UrlLocator {
            url: "https://github.com/moonrepo/plugins/releases/download/proto_tool-v0.5.3/proto_tool.wasm".into()
        }))
    }

    pub fn inherit_builtin_plugins(&mut self) {
        let is_allowed = |id: &str| match &self.settings.builtin_plugins {
            BuiltinPlugins::Enabled(state) => *state,
            BuiltinPlugins::Allowed(list) => list.iter().any(|aid| aid == id),
        };

        if !self.plugins.contains_key("asdf") && is_allowed("asdf") {
            self.plugins.insert(
                Id::raw("asdf"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/asdf_backend-v0.2.0/asdf_backend.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("bun") && is_allowed("bun") {
            self.plugins.insert(
                Id::raw("bun"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/bun_tool-v0.15.1/bun_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("deno") && is_allowed("deno") {
            self.plugins.insert(
                Id::raw("deno"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/deno_tool-v0.15.2/deno_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("go") && is_allowed("go") {
            self.plugins.insert(
                Id::raw("go"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/go_tool-v0.16.1/go_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("moon") && is_allowed("moon") {
            self.plugins.insert(
                Id::raw("moon"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/moon_tool-v0.3.1/moon_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("node") && is_allowed("node") {
            self.plugins.insert(
                Id::raw("node"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/node_tool-v0.16.1/node_tool.wasm".into()
                }))
            );
        }

        for depman in ["npm", "pnpm", "yarn"] {
            if !self.plugins.contains_key(depman) && is_allowed(depman) {
                self.plugins.insert(
                    Id::raw(depman),
                    PluginLocator::Url(Box::new(UrlLocator {
                        url: "https://github.com/moonrepo/plugins/releases/download/node_depman_tool-v0.15.1/node_depman_tool.wasm".into()
                    }))
                );
            }
        }

        if !self.plugins.contains_key("poetry") && is_allowed("poetry") {
            self.plugins.insert(
                Id::raw("poetry"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/python_poetry_tool-v0.1.2/python_poetry_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("python") && is_allowed("python") {
            self.plugins.insert(
                Id::raw("python"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/python_tool-v0.14.1/python_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("uv") && is_allowed("uv") {
            self.plugins.insert(
                Id::raw("uv"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/python_uv_tool-v0.2.1/python_uv_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("ruby") && is_allowed("ruby") {
            self.plugins.insert(
                Id::raw("ruby"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/ruby_tool-v0.2.1/ruby_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key("rust") && is_allowed("rust") {
            self.plugins.insert(
                Id::raw("rust"),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/rust_tool-v0.13.2/rust_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key(SCHEMA_PLUGIN_KEY) {
            self.plugins.insert(
                Id::raw(SCHEMA_PLUGIN_KEY),
                PluginLocator::Url(Box::new(UrlLocator {
                    url: "https://github.com/moonrepo/plugins/releases/download/schema_tool-v0.17.1/schema_tool.wasm".into()
                }))
            );
        }

        if !self.plugins.contains_key(PROTO_PLUGIN_KEY) {
            self.plugins
                .insert(Id::raw(PROTO_PLUGIN_KEY), self.builtin_proto_plugin());
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
                location: path.to_string_lossy().to_string(),
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
                    .is_some_and(|versions| versions.contains_key(field.as_str()))
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
                    location: path.to_string_lossy().to_string(),
                    error: Box::new(error),
                    help: Some(color::muted_light("https://moonrepo.dev/docs/proto/config")),
                }
                .into());
            }
        }

        // Update file paths to be absolute
        fn make_absolute<T: AsRef<OsStr>>(file: T, current_path: &Path) -> PathBuf {
            let file = PathBuf::from(file.as_ref());

            if file.is_absolute() {
                file
            } else if let Some(dir) = current_path.parent() {
                dir.join(file)
            } else {
                PathBuf::from("/").join(file)
            }
        }

        if let Some(plugins) = &mut config.plugins {
            for locator in plugins.values_mut() {
                if let PluginLocator::File(inner) = locator {
                    inner.path = Some(make_absolute(inner.get_unresolved_path(), path));
                }
            }
        }

        if let Some(settings) = &mut config.settings {
            if let Some(http) = &mut settings.http {
                if let Some(root_cert) = &mut http.root_cert {
                    *root_cert = make_absolute(&root_cert, path);
                }
            }
        }

        let push_env_file = |env_map: Option<&mut IndexMap<String, PartialEnvVar>>,
                             file_list: &mut Option<Vec<EnvFile>>,
                             extra_weight: usize|
         -> miette::Result<()> {
            if let Some(map) = env_map {
                if let Some(PartialEnvVar::Value(env_file)) = map.get(ENV_FILE_KEY) {
                    let list = file_list.get_or_insert(vec![]);
                    let env_file_path = make_absolute(env_file, path);

                    if !env_file_path.exists() {
                        return Err(ProtoConfigError::MissingEnvFile {
                            path: env_file_path,
                            config: env_file.to_owned(),
                            config_path: path.to_path_buf(),
                        }
                        .into());
                    }

                    list.push(EnvFile {
                        path: env_file_path,
                        weight: (path.to_str().map_or(0, |p| p.len()) * 10) + extra_weight,
                    });
                }

                map.shift_remove(ENV_FILE_KEY);
            }

            Ok(())
        };

        if let Some(tools) = &mut config.tools {
            for tool in tools.values_mut() {
                push_env_file(tool.env.as_mut(), &mut tool._env_files, 5)?;
            }
        }

        push_env_file(config.env.as_mut(), &mut config._env_files, 0)?;

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

    pub fn get_env_files(&self, filter_id: Option<&Id>) -> Vec<&PathBuf> {
        let mut paths: Vec<&EnvFile> = self._env_files.iter().collect();

        if let Some(id) = filter_id {
            if let Some(tool_config) = self.tools.get(id) {
                paths.extend(&tool_config._env_files);
            }
        }

        // Sort by weight so that we persist the order of env files
        // when layers across directories exist!
        paths.sort_by(|a, d| a.weight.cmp(&d.weight));

        // Then only return the paths
        paths.into_iter().map(|file| &file.path).collect()
    }

    // We don't use a `BTreeMap` for env vars, so that variable interpolation
    // and order of declaration can work correctly!
    pub fn get_env_vars(
        &self,
        filter_id: Option<&Id>,
    ) -> miette::Result<IndexMap<String, Option<String>>> {
        let env_files = self.get_env_files(filter_id);

        let mut base_vars = IndexMap::new();
        base_vars.extend(self.load_env_files(&env_files)?);
        base_vars.extend(self.env.clone());

        if let Some(id) = filter_id {
            if let Some(tool_config) = self.tools.get(id) {
                base_vars.extend(tool_config.env.clone())
            }
        }

        let mut vars = IndexMap::<String, Option<String>>::new();

        for (key, value) in base_vars {
            if key == ENV_FILE_KEY {
                continue;
            }

            let key_exists = std::env::var(&key).is_ok_and(|v| !v.is_empty());
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

            vars.insert(key, value);
        }

        Ok(vars)
    }

    pub fn load_env_files(&self, paths: &[&PathBuf]) -> miette::Result<IndexMap<String, EnvVar>> {
        let mut vars = IndexMap::default();

        let map_error = |error: dotenvy::Error, path: &Path| -> miette::Report {
            match error {
                dotenvy::Error::Io(inner) => FsError::Read {
                    path: path.to_path_buf(),
                    error: Box::new(inner),
                }
                .into(),
                other => ProtoConfigError::FailedParseEnvFile {
                    path: path.to_path_buf(),
                    error: Box::new(other),
                }
                .into(),
            }
        };

        for path in paths {
            for item in dotenvy::from_path_iter(path).map_err(|error| map_error(error, path))? {
                let (key, value) = item.map_err(|error| map_error(error, path))?;

                vars.insert(key, EnvVar::Value(value));
            }
        }

        Ok(vars)
    }
}

#[derive(Debug, Serialize)]
pub struct ProtoConfigFile {
    pub exists: bool,
    pub location: PinLocation,
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
            let is_end = end_dir.is_some_and(|end| end == dir);
            let location = if is_end {
                PinLocation::User
            } else {
                PinLocation::Local
            };

            if let Some(env) = env_mode {
                let env_path = dir.join(format!("{}.{env}", PROTO_CONFIG_NAME));

                files.push(ProtoConfigFile {
                    config: ProtoConfig::load(&env_path, false)?,
                    exists: env_path.exists(),
                    location,
                    path: env_path,
                });
            }

            let path = dir.join(PROTO_CONFIG_NAME);

            files.push(ProtoConfigFile {
                config: ProtoConfig::load(&path, false)?,
                exists: path.exists(),
                location,
                path,
            });

            if is_end {
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

            self.merge_configs(
                self.files
                    .iter()
                    .filter(|file| file.location == PinLocation::Global)
                    .collect(),
            )
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

            self.merge_configs(
                self.files
                    .iter()
                    .filter(|file| file.location != PinLocation::Global)
                    .collect(),
            )
        })
    }

    pub(crate) fn remove_proto_pins(&mut self) {
        self.files.iter_mut().for_each(|file| {
            if file.location != PinLocation::Local {
                if let Some(versions) = &mut file.config.versions {
                    versions.remove(PROTO_PLUGIN_KEY);
                }

                if let Some(unknown) = &mut file.config.unknown {
                    unknown.remove(PROTO_PLUGIN_KEY);
                }
            }
        });
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
        config.setup_env_vars();

        debug!("Merged {} configs", count);

        Ok(config)
    }
}
