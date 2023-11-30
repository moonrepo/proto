use miette::IntoDiagnostic;
use once_cell::sync::OnceCell;
use schematic::{derive_enum, env, Config, ConfigEnum, ConfigLoader, Format, PartialConfig};
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
                    url: "https://github.com/moonrepo/bun-plugin/releases/download/v0.5.0/bun_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("deno") {
            self.plugins.insert(
                Id::raw("deno"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/deno-plugin/releases/download/v0.5.0/deno_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("go") {
            self.plugins.insert(
                Id::raw("go"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/go-plugin/releases/download/v0.5.0/go_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("node") {
            self.plugins.insert(
                Id::raw("node"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/node-plugin/releases/download/v0.5.3/node_plugin.wasm".into()
                }
            );
        }

        for depman in ["npm", "pnpm", "yarn"] {
            if !self.plugins.contains_key(depman) {
                self.plugins.insert(
                    Id::raw(depman),
                    PluginLocator::SourceUrl {
                        url: "https://github.com/moonrepo/node-plugin/releases/download/v0.5.3/node_depman_plugin.wasm".into()
                    }
                );
            }
        }

        if !self.plugins.contains_key("python") {
            self.plugins.insert(
                Id::raw("python"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/python-plugin/releases/download/v0.3.0/python_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("rust") {
            self.plugins.insert(
                Id::raw("rust"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/rust-plugin/releases/download/v0.4.0/rust_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key(SCHEMA_PLUGIN_KEY) {
            self.plugins.insert(
                Id::raw(SCHEMA_PLUGIN_KEY),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/schema-plugin/releases/download/v0.5.0/schema_plugin.wasm".into()
                }
            );
        }
    }
}

pub struct ProtoConfigManager {
    pub files: BTreeMap<PathBuf, PartialProtoConfig>,

    merged_config: Arc<OnceCell<ProtoConfig>>,
}

impl ProtoConfigManager {
    pub fn load(start_dir: &Path, end_dir: Option<&Path>) -> miette::Result<Self> {
        trace!("Traversing upwards and loading {} files", PROTO_CONFIG_NAME);

        let mut current_dir = Some(start_dir);
        let mut files = BTreeMap::new();

        while let Some(dir) = current_dir {
            if !files.contains_key(dir) {
                files.insert(dir.to_path_buf(), Self::load_from(dir)?);
            }

            if end_dir.is_some_and(|end| end == dir) {
                break;
            }

            current_dir = dir.parent();
        }

        Ok(Self {
            files,
            merged_config: Arc::new(OnceCell::new()),
        })
    }

    #[tracing::instrument(skip_all)]
    pub fn load_from<P: AsRef<Path>>(dir: P) -> miette::Result<PartialProtoConfig> {
        let dir = dir.as_ref();
        let path = dir.join(PROTO_CONFIG_NAME);

        if !path.exists() {
            return Ok(PartialProtoConfig::default());
        }

        debug!(file = ?path, "Loading {}", PROTO_CONFIG_NAME);

        let mut config = ConfigLoader::<ProtoConfig>::new()
            .code(fs::read_file_with_lock(&path)?, Format::Toml)?
            .load_partial(&())?;

        let make_absolute = |file: &mut PathBuf| {
            if file.is_absolute() {
                file.to_owned()
            } else {
                dir.join(file)
            }
        };

        // Update plugin file paths to be absolute
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

    #[tracing::instrument(skip_all)]
    pub fn save_to(dir: &Path, config: PartialProtoConfig) -> miette::Result<PathBuf> {
        let path = dir.join(PROTO_CONFIG_NAME);

        fs::write_file_with_lock(&path, toml::to_string_pretty(&config).into_diagnostic()?)?;

        Ok(path)
    }

    pub fn get_merged_config(&self) -> miette::Result<&ProtoConfig> {
        self.merged_config.get_or_try_init(|| {
            let mut partial = PartialProtoConfig::default();
            let context = &();

            for file in self.files.values() {
                partial.merge(context, file.to_owned())?;
            }

            let mut config = ProtoConfig::from_partial(partial.finalize(context)?);
            config.inherit_builtin_plugins();

            Ok(config)
        })
    }
}
