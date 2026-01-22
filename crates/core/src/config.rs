use crate::config_error::ProtoConfigError;
use crate::helpers::ENV_VAR_SUB;
use crate::tool_context::ToolContext;
use crate::tool_spec::ToolSpec;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use schematic::{
    Config, ConfigError, ConfigLoader, PartialConfig, Path as ErrorPath, ValidateError,
    ValidatorError, merge,
};
use serde::Serialize;
use starbase_styles::color;
use starbase_utils::fs::FsError;
use starbase_utils::toml::TomlValue;
use starbase_utils::{fs, toml};
use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;
use tracing::{debug, instrument, trace};
use warpgate::{Id, PluginLocator, find_debug_locator_with_url_fallback};

// Re-export settings from here!
pub use crate::settings::*;

pub const PROTO_CONFIG_NAME: &str = ".prototools";
pub const SCHEMA_PLUGIN_KEY: &str = "internal-schema";
pub const PROTO_PLUGIN_KEY: &str = "proto";
pub const ENV_FILE_KEY: &str = "file";

#[derive(Clone, Config, Debug, Serialize)]
#[config(allow_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoConfig {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[setting(nested, merge = merge_partials_iter)]
    pub backends: BTreeMap<String, ProtoBackendConfig>,

    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    #[setting(nested, merge = merge_iter)]
    pub env: IndexMap<String, EnvVar>,

    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[setting(nested, merge = merge_partials_iter)]
    pub tools: BTreeMap<String, ProtoToolConfig>,

    #[setting(nested)]
    pub plugins: ProtoPluginsConfig,

    #[setting(nested)]
    pub settings: ProtoSettingsConfig,

    #[serde(flatten)]
    #[setting(merge = merge_iter)]
    pub versions: BTreeMap<ToolContext, ToolSpec>,

    #[serde(flatten, skip_serializing)]
    #[setting(merge = merge_iter)]
    pub unknown: FxHashMap<String, TomlValue>,

    #[serde(skip)]
    #[setting(exclude, merge = merge::append_vec)]
    pub(crate) _env_files: Vec<EnvFile>,
}

impl ProtoConfig {
    pub fn get_backend_config(&self, context: &ToolContext) -> Option<&ProtoBackendConfig> {
        context
            .backend
            .as_ref()
            .and_then(|id| self.backends.get(id.as_str()))
    }

    pub fn get_tool_config(&self, context: &ToolContext) -> Option<&ProtoToolConfig> {
        // To avoid ID collisions between tools and backend managed tools,
        // the latter's configuration must include the backend prefix.
        // For example, "npm:node" instead of just "node" (collision).
        if context.backend.is_some() {
            self.tools
                .get(context.as_str())
                // TODO remove in v0.54
                .or_else(|| self.tools.get(context.id.as_str()))
        } else {
            self.tools.get(context.as_str())
        }
    }

    pub fn setup_env_vars(&self) {
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

    pub fn builtin_plugins(&self) -> ProtoPluginsConfig {
        let mut config = ProtoConfig::default();

        // Inherit this setting in case builtins have been disabled
        config.settings.builtin_plugins = self.settings.builtin_plugins.clone();

        // Then inherit all the available builtins
        config.inherit_builtin_plugins();

        config.plugins
    }

    pub fn builtin_proto_plugin(&self) -> PluginLocator {
        find_debug_locator_with_url_fallback("proto_tool", "0.5.5")
    }

    pub fn builtin_schema_plugin(&self) -> PluginLocator {
        find_debug_locator_with_url_fallback("schema_tool", "0.17.7")
    }

    pub fn inherit_builtin_plugins(&mut self) {
        let is_allowed = |id: &str| match &self.settings.builtin_plugins {
            BuiltinPlugins::Enabled(state) => *state,
            BuiltinPlugins::Allowed(list) => list.iter().any(|aid| aid == id),
        };

        let proto_locator = self.builtin_proto_plugin();
        let schema_locator = self.builtin_schema_plugin();
        let backends = &mut self.plugins.backends;
        let tools = &mut self.plugins.tools;

        if !backends.contains_key("asdf") && is_allowed("asdf") {
            backends.insert(
                Id::raw("asdf"),
                find_debug_locator_with_url_fallback("asdf_backend", "0.3.2"),
            );
        }

        if !tools.contains_key("bun") && is_allowed("bun") {
            tools.insert(
                Id::raw("bun"),
                find_debug_locator_with_url_fallback("bun_tool", "0.16.4"),
            );
        }

        if !tools.contains_key("deno") && is_allowed("deno") {
            tools.insert(
                Id::raw("deno"),
                find_debug_locator_with_url_fallback("deno_tool", "0.15.7"),
            );
        }

        if !tools.contains_key("go") && is_allowed("go") {
            tools.insert(
                Id::raw("go"),
                find_debug_locator_with_url_fallback("go_tool", "0.16.5"),
            );
        }

        if !tools.contains_key("moon") && is_allowed("moon") {
            tools.insert(
                Id::raw("moon"),
                find_debug_locator_with_url_fallback("moon_tool", "0.4.0"),
            );
        }

        if !tools.contains_key("node") && is_allowed("node") {
            tools.insert(
                Id::raw("node"),
                find_debug_locator_with_url_fallback("node_tool", "0.17.5"),
            );
        }

        for depman in ["npm", "pnpm", "yarn"] {
            if !tools.contains_key(depman) && is_allowed(depman) {
                tools.insert(
                    Id::raw(depman),
                    find_debug_locator_with_url_fallback("node_depman_tool", "0.17.1"),
                );
            }
        }

        if !tools.contains_key("poetry") && is_allowed("poetry") {
            tools.insert(
                Id::raw("poetry"),
                find_debug_locator_with_url_fallback("python_poetry_tool", "0.1.5"),
            );
        }

        if !tools.contains_key("python") && is_allowed("python") {
            tools.insert(
                Id::raw("python"),
                find_debug_locator_with_url_fallback("python_tool", "0.14.5"),
            );
        }

        if !tools.contains_key("uv") && is_allowed("uv") {
            tools.insert(
                Id::raw("uv"),
                find_debug_locator_with_url_fallback("python_uv_tool", "0.3.1"),
            );
        }

        if !tools.contains_key("ruby") && is_allowed("ruby") {
            tools.insert(
                Id::raw("ruby"),
                find_debug_locator_with_url_fallback("ruby_tool", "0.2.5"),
            );
        }

        if !tools.contains_key("rust") && is_allowed("rust") {
            tools.insert(
                Id::raw("rust"),
                find_debug_locator_with_url_fallback("rust_tool", "0.13.7"),
            );
        }

        if !tools.contains_key(SCHEMA_PLUGIN_KEY) {
            tools.insert(Id::raw(SCHEMA_PLUGIN_KEY), schema_locator);
        }

        if !tools.contains_key(PROTO_PLUGIN_KEY) {
            tools.insert(Id::raw(PROTO_PLUGIN_KEY), proto_locator);
        }

        #[cfg(all(any(debug_assertions, test), feature = "test-plugins"))]
        {
            let locator = warpgate::find_debug_locator("proto_mocked_tool")
                .expect("Test plugins not available. Run `just build-wasm` to build them!");

            tools.insert(Id::raw("moonbase"), locator.clone());
            tools.insert(Id::raw("moonstone"), locator.clone());
            tools.insert(Id::raw("protoform"), locator.clone());
            tools.insert(Id::raw("protostar"), locator);
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
            .code(config_content, format!("{}.toml", PROTO_CONFIG_NAME))?
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
            let mut log = true;

            let abs_file = if file.is_absolute() {
                log = false;

                file.clone()
            } else if let Some(dir) = current_path.parent() {
                dir.join(&file)
            } else {
                std::env::current_dir().unwrap().join(&file)
            };

            if log {
                trace!(
                    in_file = ?file,
                    out_file = ?abs_file,
                    "Making file path absolute",
                );
            }

            abs_file
        }

        if let Some(plugins) = &mut config.plugins {
            if let Some(backends) = &mut plugins.backends {
                for locator in backends.values_mut() {
                    if let PluginLocator::File(inner) = locator {
                        inner.path = Some(make_absolute(inner.get_unresolved_path(), path));
                    }
                }
            }

            if let Some(tools) = &mut plugins.tools {
                for locator in tools.values_mut() {
                    if let PluginLocator::File(inner) = locator {
                        inner.path = Some(make_absolute(inner.get_unresolved_path(), path));
                    }
                }
            }

            if let Some(tools) = &mut plugins.legacy {
                for locator in tools.values_mut() {
                    if let PluginLocator::File(inner) = locator {
                        inner.path = Some(make_absolute(inner.get_unresolved_path(), path));
                    }
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

                    if env_file_path.exists() {
                        list.push(EnvFile {
                            path: env_file_path,
                            weight: (path.to_str().map_or(0, |p| p.len()) * 10) + extra_weight,
                        });
                    }
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

        if let Some(backends) = &mut config.backends {
            for backend in backends.values_mut() {
                push_env_file(backend.env.as_mut(), &mut backend._env_files, 3)?;
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

        if let Some(context) = options.context {
            if let Some(backend_config) = self.get_backend_config(context) {
                paths.extend(&backend_config._env_files);
            }

            if let Some(tool_config) = self.get_tool_config(context) {
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

        if let Some(context) = options.context {
            if let Some(backend_config) = self.get_backend_config(context) {
                base_vars.extend(backend_config.env.clone())
            }

            if let Some(tool_config) = self.get_tool_config(context) {
                base_vars.extend(tool_config.env.clone())
            }
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
pub struct ProtoConfigEnvOptions<'ctx> {
    pub context: Option<&'ctx ToolContext>,
    pub check_process: bool,
    pub include_shared: bool,
}
