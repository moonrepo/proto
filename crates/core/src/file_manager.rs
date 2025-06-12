use crate::config::*;
use crate::config_error::ProtoConfigError;
use once_cell::sync::OnceCell;
use schematic::{Config, PartialConfig};
use serde::Serialize;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

#[derive(Debug, Serialize)]
pub struct ProtoFile {
    pub exists: bool,
    pub location: PinLocation,
    pub path: PathBuf,
    pub config: PartialProtoConfig,
}

#[derive(Debug)]
pub struct ProtoFileManager {
    // Paths are sorted from current working directory,
    // up until the root or user directory, whichever is first.
    // The special `~/.proto/.prototools` config is always
    // loaded last, and is the last entry in the list.
    // For directories without a config, we still insert
    // an empty entry. This helps with traversal logic.
    pub files: Vec<ProtoFile>,

    all_config: Arc<OnceCell<ProtoConfig>>,
    all_config_no_global: Arc<OnceCell<ProtoConfig>>,
    global_config: Arc<OnceCell<ProtoConfig>>,
    local_config: Arc<OnceCell<ProtoConfig>>,
}

impl ProtoFileManager {
    pub fn load(
        start_dir: impl AsRef<Path>,
        end_dir: Option<&Path>,
        env_mode: Option<&String>,
    ) -> Result<Self, ProtoConfigError> {
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

                files.push(ProtoFile {
                    config: ProtoConfig::load(&env_path, false)?,
                    exists: env_path.exists(),
                    location,
                    path: env_path,
                });
            }

            let path = dir.join(PROTO_CONFIG_NAME);

            files.push(ProtoFile {
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

    pub fn get_global_config(&self) -> Result<&ProtoConfig, ProtoConfigError> {
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

    pub fn get_local_config(&self, cwd: &Path) -> Result<&ProtoConfig, ProtoConfigError> {
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

    pub fn get_merged_config(&self) -> Result<&ProtoConfig, ProtoConfigError> {
        self.all_config.get_or_try_init(|| {
            debug!("Merging loaded configs with global");

            self.merge_configs(self.files.iter().collect())
        })
    }

    pub fn get_merged_config_without_global(&self) -> Result<&ProtoConfig, ProtoConfigError> {
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

    fn merge_configs(&self, files: Vec<&ProtoFile>) -> Result<ProtoConfig, ProtoConfigError> {
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
