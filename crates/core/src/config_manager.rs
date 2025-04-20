use crate::config::*;
use crate::lockfile::*;
use once_cell::sync::OnceCell;
use schematic::{Config, PartialConfig};
use serde::Serialize;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Serialize)]
pub struct ProtoConfigFile {
    pub exists: bool,
    pub location: PinLocation,
    pub lockfile: Option<ProtoLockfile>,
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

    all_config: OnceCell<ProtoConfig>,
    all_config_no_global: OnceCell<ProtoConfig>,
    global_config: OnceCell<ProtoConfig>,
    local_config: OnceCell<ProtoConfig>,
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
                    lockfile: None,
                    path: env_path,
                });
            }

            let config_path = dir.join(PROTO_CONFIG_NAME);
            let lock_path = dir.join(PROTO_LOCKFILE_NAME);

            files.push(ProtoConfigFile {
                config: ProtoConfig::load(&config_path, false)?,
                exists: config_path.exists(),
                location,
                lockfile: if lock_path.exists() {
                    Some(ProtoLockfile::load(lock_path)?)
                } else {
                    None
                },
                path: config_path,
            });

            if is_end {
                break;
            }

            current_dir = dir.parent();
        }

        Ok(Self {
            files,
            all_config: OnceCell::new(),
            all_config_no_global: OnceCell::new(),
            global_config: OnceCell::new(),
            local_config: OnceCell::new(),
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
