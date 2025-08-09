use crate::config_error::ProtoConfigError;
use crate::helpers::ENV_VAR_SUB;
use crate::tool_context::ToolContext;
use crate::tool_spec::ToolSpec;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use schematic::{
    Config, ConfigEnum, ConfigError, ConfigLoader, DefaultValueResult, Format, MergeError,
    MergeResult, PartialConfig, Path as ErrorPath, RegexSetting, ValidateError, ValidateResult,
    ValidatorError, derive_enum, env, merge,
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
use system_env::{SystemOS, SystemPackageManager};
use toml_edit::DocumentMut;
use tracing::{debug, instrument};
use warpgate::{HttpOptions, Id, PluginLocator, RegistryConfig, UrlLocator};

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

fn validate_default_registry(
    value: &str,
    partial: &PartialProtoSettingsConfig,
    _context: &(),
    finalize: bool,
) -> ValidateResult {
    if finalize
        && let Some(registries) = &partial.registries
        && !registries.iter().any(|reg| reg.registry == value)
    {
        let existing = registries
            .iter()
            .map(|reg| reg.registry.clone())
            .collect::<Vec<_>>()
            .join(", ");

        return Err(ValidateError::new(format!(
            "default registry {value} does not exist, available registries: {existing}"
        )));
    }

    Ok(())
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[setting(env = "PROTO_BUILD_EXCLUDE_PACKAGES", parse_env = env::split_comma)]
    pub exclude_packages: Vec<String>,

    #[setting(default = true, env = "PROTO_BUILD_INSTALL_SYSTEM_PACKAGES", parse_env = env::parse_bool)]
    pub install_system_packages: bool,

    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub system_package_manager: FxHashMap<SystemOS, Option<SystemPackageManager>>,

    #[setting(env = "PROTO_BUILD_WRITE_LOG_FILE", parse_env = env::parse_bool)]
    pub write_log_file: bool,
}

#[derive(Clone, Config, Debug, Serialize)]
#[config(allow_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoToolConfig {
    #[setting(merge = merge::merge_btreemap)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub aliases: BTreeMap<String, ToolSpec>,

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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[setting(env = "PROTO_OFFLINE_HOSTS", parse_env = env::split_comma)]
    pub custom_hosts: Vec<String>,

    #[setting(env = "PROTO_OFFLINE_OVERRIDE_HOSTS", parse_env = env::parse_bool)]
    pub override_default_hosts: bool,

    #[setting(default = 750, env = "PROTO_OFFLINE_TIMEOUT")]
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

    #[setting(env = "PROTO_CACHE_DURATION")]
    pub cache_duration: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[setting(env = "PROTO_DEFAULT_REGISTRY", validate = validate_default_registry)]
    pub default_registry: Option<String>,

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

    #[setting(merge = merge::append_vec)]
    #[serde(alias = "unstable-registries", skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<RegistryConfig>,

    #[setting(default = true, env = "PROTO_TELEMETRY", parse_env = env::parse_bool)]
    pub telemetry: bool,

    #[setting(merge = merge_indexmap)]
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub url_rewrites: IndexMap<RegexSetting, String>,
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
    pub versions: BTreeMap<ToolContext, ToolSpec>,

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
        find_debug_locator_with_version("proto_tool", "0.5.4")
    }

    pub fn inherit_builtin_plugins(&mut self) {
        let is_allowed = |id: &str| match &self.settings.builtin_plugins {
            BuiltinPlugins::Enabled(state) => *state,
            BuiltinPlugins::Allowed(list) => list.iter().any(|aid| aid == id),
        };

        if !self.plugins.contains_key("asdf") && is_allowed("asdf") {
            self.plugins.insert(
                Id::raw("asdf"),
                find_debug_locator_with_version("asdf_backend", "0.3.1"),
            );
        }

        if !self.plugins.contains_key("bun") && is_allowed("bun") {
            self.plugins.insert(
                Id::raw("bun"),
                find_debug_locator_with_version("bun_tool", "0.16.1"),
            );
        }

        if !self.plugins.contains_key("deno") && is_allowed("deno") {
            self.plugins.insert(
                Id::raw("deno"),
                find_debug_locator_with_version("deno_tool", "0.15.5"),
            );
        }

        if !self.plugins.contains_key("go") && is_allowed("go") {
            self.plugins.insert(
                Id::raw("go"),
                find_debug_locator_with_version("go_tool", "0.16.3"),
            );
        }

        if !self.plugins.contains_key("moon") && is_allowed("moon") {
            self.plugins.insert(
                Id::raw("moon"),
                find_debug_locator_with_version("moon_tool", "0.3.3"),
            );
        }

        if !self.plugins.contains_key("node") && is_allowed("node") {
            self.plugins.insert(
                Id::raw("node"),
                find_debug_locator_with_version("node_tool", "0.17.1"),
            );
        }

        for depman in ["npm", "pnpm", "yarn"] {
            if !self.plugins.contains_key(depman) && is_allowed(depman) {
                self.plugins.insert(
                    Id::raw(depman),
                    find_debug_locator_with_version("node_depman_tool", "0.16.1"),
                );
            }
        }

        if !self.plugins.contains_key("poetry") && is_allowed("poetry") {
            self.plugins.insert(
                Id::raw("poetry"),
                find_debug_locator_with_version("python_poetry_tool", "0.1.4"),
            );
        }

        if !self.plugins.contains_key("python") && is_allowed("python") {
            self.plugins.insert(
                Id::raw("python"),
                find_debug_locator_with_version("python_tool", "0.14.3"),
            );
        }

        if !self.plugins.contains_key("uv") && is_allowed("uv") {
            self.plugins.insert(
                Id::raw("uv"),
                find_debug_locator_with_version("python_uv_tool", "0.3.1"),
            );
        }

        if !self.plugins.contains_key("ruby") && is_allowed("ruby") {
            self.plugins.insert(
                Id::raw("ruby"),
                find_debug_locator_with_version("ruby_tool", "0.2.3"),
            );
        }

        if !self.plugins.contains_key("rust") && is_allowed("rust") {
            self.plugins.insert(
                Id::raw("rust"),
                find_debug_locator_with_version("rust_tool", "0.13.4"),
            );
        }

        if !self.plugins.contains_key(SCHEMA_PLUGIN_KEY) {
            self.plugins.insert(
                Id::raw(SCHEMA_PLUGIN_KEY),
                find_debug_locator_with_version("schema_tool", "0.17.5"),
            );
        }

        if !self.plugins.contains_key(PROTO_PLUGIN_KEY) {
            self.plugins
                .insert(Id::raw(PROTO_PLUGIN_KEY), self.builtin_proto_plugin());
        }

        #[cfg(all(any(debug_assertions, test), feature = "test-plugins"))]
        {
            let locator = find_debug_locator("proto_mocked_tool")
                .expect("Test plugins not available. Run `just build-wasm` to build them!");

            self.plugins.insert(Id::raw("moonbase"), locator.clone());
            self.plugins.insert(Id::raw("moonstone"), locator.clone());
            self.plugins.insert(Id::raw("protoform"), locator.clone());
            self.plugins.insert(Id::raw("protostar"), locator);
        }
    }

    pub fn load_from<P: AsRef<Path>>(
        dir: P,
        with_lock: bool,
    ) -> Result<PartialProtoConfig, ProtoConfigError> {
        Self::load(Self::resolve_path(dir), with_lock)
    }

    #[instrument(name = "load_config")]
    pub fn load<P: AsRef<Path> + Debug>(
        path: P,
        with_lock: bool,
    ) -> Result<PartialProtoConfig, ProtoConfigError> {
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
                let message = if value.is_array() || value.is_table() {
                    format!("unknown field `{field}`")
                } else {
                    match ToolContext::parse(field) {
                        Ok(context) => {
                            // Versions show up in both flattened maps...
                            if config
                                .versions
                                .as_ref()
                                .is_some_and(|versions| versions.contains_key(&context))
                            {
                                continue;
                            } else {
                                format!("invalid version value `{value}`")
                            }
                        }
                        Err(error) => error.to_string(),
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

        if let Some(settings) = &mut config.settings
            && let Some(http) = &mut settings.http
            && let Some(root_cert) = &mut http.root_cert
        {
            *root_cert = make_absolute(&root_cert, path);
        }

        let push_env_file = |env_map: Option<&mut IndexMap<String, PartialEnvVar>>,
                             file_list: &mut Option<Vec<EnvFile>>,
                             extra_weight: usize|
         -> Result<(), ProtoConfigError> {
            if let Some(map) = env_map {
                if let Some(PartialEnvVar::Value(env_file)) = map.get(ENV_FILE_KEY) {
                    let list = file_list.get_or_insert(vec![]);
                    let env_file_path = make_absolute(env_file, path);

                    if !env_file_path.exists() {
                        return Err(ProtoConfigError::MissingEnvFile {
                            path: env_file_path,
                            config: env_file.to_owned(),
                            config_path: path.to_path_buf(),
                        });
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
    pub fn save_to<P: AsRef<Path> + Debug, C: AsRef<[u8]>>(
        dir: P,
        config: C,
    ) -> Result<PathBuf, ProtoConfigError> {
        let file = Self::resolve_path(dir);

        fs::write_file_with_lock(&file, &config)?;

        Ok(file)
    }

    pub fn save_partial_to<P: AsRef<Path> + Debug>(
        dir: P,
        config: PartialProtoConfig,
    ) -> Result<PathBuf, ProtoConfigError> {
        Self::save_to(dir, toml::format(&config, true)?)
    }

    pub fn update<P: AsRef<Path>, F: FnOnce(&mut PartialProtoConfig)>(
        dir: P,
        op: F,
    ) -> Result<PathBuf, ProtoConfigError> {
        let dir = dir.as_ref();
        let mut config = Self::load_from(dir, true)?;

        op(&mut config);

        Self::save_partial_to(dir, config)
    }

    pub fn update_document<P: AsRef<Path>, F: FnOnce(&mut DocumentMut)>(
        dir: P,
        op: F,
    ) -> Result<PathBuf, ProtoConfigError> {
        let path = Self::resolve_path(dir);
        let config = if path.exists() {
            fs::read_file_with_lock(&path)?
        } else {
            String::new()
        };
        let mut document =
            config
                .parse::<DocumentMut>()
                .map_err(|error| ProtoConfigError::FailedUpdate {
                    path: path.clone(),
                    error: Box::new(error),
                })?;

        op(&mut document);

        Self::save_to(path, document.to_string())
    }

    pub fn get_env_files(&self, options: ProtoConfigEnvOptions) -> Vec<&PathBuf> {
        let mut paths: Vec<&EnvFile> = vec![];

        if options.include_shared {
            paths.extend(&self._env_files);
        }

        if let Some(tool_id) = &options.tool_id
            && let Some(tool_config) = self.tools.get(tool_id)
        {
            paths.extend(&tool_config._env_files);
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
        options: ProtoConfigEnvOptions,
    ) -> Result<IndexMap<String, Option<String>>, ProtoConfigError> {
        let env_files = self.get_env_files(options.clone());

        let mut base_vars = IndexMap::new();

        if !env_files.is_empty() {
            base_vars.extend(self.load_env_files(&env_files)?);
        }

        if options.include_shared {
            base_vars.extend(self.env.clone());
        }

        if let Some(tool_id) = &options.tool_id
            && let Some(tool_config) = self.tools.get(tool_id)
        {
            base_vars.extend(tool_config.env.clone())
        }

        let mut vars = IndexMap::<String, Option<String>>::new();

        for (key, value) in base_vars {
            if key == ENV_FILE_KEY {
                continue;
            }

            let key_exists =
                options.check_process && std::env::var(&key).is_ok_and(|v| !v.is_empty());
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

    pub fn load_env_files(
        &self,
        paths: &[&PathBuf],
    ) -> Result<IndexMap<String, EnvVar>, ProtoConfigError> {
        let mut vars = IndexMap::default();

        let map_error = |error: dotenvy::Error, path: &Path| -> ProtoConfigError {
            match error {
                dotenvy::Error::Io(inner) => ProtoConfigError::Fs(Box::new(FsError::Read {
                    path: path.to_path_buf(),
                    error: Box::new(inner),
                })),
                other => ProtoConfigError::FailedParseEnvFile {
                    path: path.to_path_buf(),
                    error: Box::new(other),
                },
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

    pub fn rewrite_url(&self, url: impl AsRef<str>) -> String {
        let mut url = url.as_ref().to_owned();

        for (pattern, replacement) in &self.settings.url_rewrites {
            url = pattern.replace_all(&url, replacement).to_string();
        }

        url
    }

    fn resolve_path(path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();

        if path.ends_with(PROTO_CONFIG_NAME) {
            path.to_path_buf()
        } else {
            path.join(PROTO_CONFIG_NAME)
        }
    }
}

#[derive(Clone, Default)]
pub struct ProtoConfigEnvOptions {
    pub check_process: bool,
    pub include_shared: bool,
    pub tool_id: Option<Id>,
}

#[cfg(any(debug_assertions, test))]
fn find_debug_locator(name: &str) -> Option<PluginLocator> {
    use warpgate::{FileLocator, test_utils::find_wasm_file_with_name};

    find_wasm_file_with_name(name).map(|wasm_path| {
        PluginLocator::File(Box::new(FileLocator {
            file: wasm_path.to_string_lossy().to_string(),
            path: Some(wasm_path),
        }))
    })
}

#[cfg(not(any(debug_assertions, test)))]
fn find_debug_locator(_name: &str) -> Option<PluginLocator> {
    None
}

fn find_debug_locator_with_version(name: &str, version: &str) -> PluginLocator {
    find_debug_locator(name).unwrap_or_else(|| {
        PluginLocator::Url(Box::new(UrlLocator {
            url: format!(
                "https://github.com/moonrepo/plugins/releases/download/{name}-v{version}/{name}.wasm"
            ),
        }))
    })
}
