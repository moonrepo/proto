use crate::config::*;
use crate::config_error::ProtoConfigError;
use crate::id::Id;
use crate::lockfile::*;
use crate::tool_context::ToolContext;
use once_cell::sync::OnceCell;
use schematic::{Config, PartialConfig};
use serde::Serialize;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::debug;

#[derive(Debug, Serialize)]
pub struct ProtoConfigFile {
    pub exists: bool,
    pub path: PathBuf,
    pub config: PartialProtoConfig,
}

#[derive(Debug, Serialize)]
pub struct ProtoDirEntry {
    pub path: PathBuf,
    pub location: PinLocation,
    pub configs: Vec<ProtoConfigFile>,
    pub locked: bool,
}

#[derive(Debug)]
pub struct ProtoFileManager {
    // Paths are sorted from current working directory,
    // up until the root or user directory, whichever is first.
    // The special `~/.proto/.prototools` config is always
    // loaded last, and is the last entry in the list.
    // For directories without a config, we still insert
    // an empty entry. This helps with traversal logic.
    pub entries: Vec<ProtoDirEntry>,

    all_config: Arc<OnceCell<ProtoConfig>>,
    all_config_no_global: Arc<OnceCell<ProtoConfig>>,
    global_config: Arc<OnceCell<ProtoConfig>>,
    local_config: Arc<OnceCell<ProtoConfig>>,

    // Lockfiles keyed by the directory of the config
    // in which they were enabled
    locks: BTreeMap<PathBuf, Arc<RwLock<ProtoLock>>>,
}

impl ProtoFileManager {
    pub fn load(
        start_dir: impl AsRef<Path>,
        end_dir: Option<&Path>,
        env_mode: Option<&String>,
    ) -> Result<Self, ProtoConfigError> {
        let mut current_dir = Some(start_dir.as_ref());
        let mut entries = vec![];
        let mut locks = BTreeMap::default();

        while let Some(dir) = current_dir {
            let mut configs = vec![];
            let is_end = end_dir.is_some_and(|end| end == dir);
            let location = if is_end {
                PinLocation::User
            } else {
                PinLocation::Local
            };

            if let Some(env) = env_mode {
                let env_path = dir.join(format!("{PROTO_CONFIG_NAME}.{env}"));

                configs.push(ProtoConfigFile {
                    config: ProtoConfig::load(&env_path, false)?,
                    exists: env_path.exists(),
                    path: env_path,
                });
            }

            let lock_path = dir.join(PROTO_LOCK_NAME);
            let path = dir.join(PROTO_CONFIG_NAME);

            configs.push(ProtoConfigFile {
                config: ProtoConfig::load(&path, false)?,
                exists: path.exists(),
                path,
            });

            // Only load the lockfile if any of the configs
            // in the current directory are enabled
            let load_lockfile = location == PinLocation::Local
                && configs.iter().any(|file| {
                    file.config
                        .settings
                        .as_ref()
                        .and_then(|settings| settings.lockfile)
                        .unwrap_or(false)
                });

            if load_lockfile {
                locks.insert(
                    dir.to_path_buf(),
                    Arc::new(RwLock::new(ProtoLock::load_from(dir)?)),
                );
            } else if lock_path.exists() {
                fs::remove_file(lock_path)?;
            }

            entries.push(ProtoDirEntry {
                path: dir.to_path_buf(),
                location,
                configs,
                locked: load_lockfile,
            });

            if is_end {
                break;
            }

            current_dir = dir.parent();
        }

        Ok(Self {
            entries,
            all_config: Arc::new(OnceCell::new()),
            all_config_no_global: Arc::new(OnceCell::new()),
            global_config: Arc::new(OnceCell::new()),
            local_config: Arc::new(OnceCell::new()),
            locks,
        })
    }

    /// Return the directory of the lockfile that applies to the provided tool,
    /// if there is one. A lockfile is scoped to the config in which it was
    /// enabled: it only applies to tools with versions defined in that config,
    /// or ad-hoc installs within that config's directory scope, and never to
    /// tools defined in nested (or global/user) configs.
    pub fn get_locked_dir(&self, context: &ToolContext) -> Option<&Path> {
        // The closest config that defines a version for the tool
        // owns its lock records
        for entry in &self.entries {
            if entry.configs.iter().any(|file| {
                file.exists
                    && file
                        .config
                        .versions
                        .as_ref()
                        .is_some_and(|versions| versions.contains_key(context))
            }) {
                return entry.locked.then_some(entry.path.as_path());
            }
        }

        // When not defined in any config, treat the tool as an ad-hoc
        // install owned by the closest directory with a config
        for entry in &self.entries {
            if entry.location == PinLocation::Local && entry.configs.iter().any(|file| file.exists)
            {
                return entry.locked.then_some(entry.path.as_path());
            }
        }

        None
    }

    pub fn get_lock(
        &self,
        dir: &Path,
    ) -> Result<Option<RwLockReadGuard<'_, ProtoLock>>, ProtoConfigError> {
        self.locks
            .get(dir)
            .map(|lock| {
                lock.read()
                    .map_err(|_| ProtoConfigError::FailedLockfileLock)
            })
            .transpose()
    }

    pub fn get_lock_mut(
        &self,
        dir: &Path,
    ) -> Result<Option<RwLockWriteGuard<'_, ProtoLock>>, ProtoConfigError> {
        self.locks
            .get(dir)
            .map(|lock| {
                lock.write()
                    .map_err(|_| ProtoConfigError::FailedLockfileLock)
            })
            .transpose()
    }

    pub fn get_config_files(&self) -> Vec<&ProtoConfigFile> {
        self.entries.iter().flat_map(|dir| &dir.configs).collect()
    }

    pub fn get_global_config(&self) -> Result<&ProtoConfig, ProtoConfigError> {
        self.global_config.get_or_try_init(|| {
            debug!("Loading global config only");

            self.merge_configs(
                self.entries
                    .iter()
                    .filter_map(|dir| {
                        if dir.location == PinLocation::Global {
                            Some(dir.configs.iter())
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect(),
            )
        })
    }

    pub fn get_local_config(&self, cwd: &Path) -> Result<&ProtoConfig, ProtoConfigError> {
        self.local_config.get_or_try_init(|| {
            debug!("Loading local config only");

            self.merge_configs(
                self.entries
                    .iter()
                    .filter_map(|dir| {
                        if dir.path == cwd {
                            Some(dir.configs.iter())
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect(),
            )
        })
    }

    pub fn get_merged_config(&self) -> Result<&ProtoConfig, ProtoConfigError> {
        self.all_config.get_or_try_init(|| {
            debug!("Merging loaded configs with global");

            self.merge_configs(
                self.entries
                    .iter()
                    .flat_map(|dir| dir.configs.iter())
                    .collect(),
            )
        })
    }

    pub fn get_merged_config_without_global(&self) -> Result<&ProtoConfig, ProtoConfigError> {
        self.all_config_no_global.get_or_try_init(|| {
            debug!("Merging loaded configs without global");

            self.merge_configs(
                self.entries
                    .iter()
                    .filter_map(|dir| {
                        if dir.location != PinLocation::Global {
                            Some(dir.configs.iter())
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect(),
            )
        })
    }

    pub(crate) fn remove_proto_pins(&mut self) {
        let context = ToolContext::new(Id::raw(PROTO_PLUGIN_KEY));

        self.entries.iter_mut().for_each(|dir| {
            if dir.location != PinLocation::Local {
                dir.configs.iter_mut().for_each(|file| {
                    if let Some(versions) = &mut file.config.versions {
                        versions.remove(&context);
                    }

                    if let Some(unknown) = &mut file.config.unknown {
                        unknown.remove(PROTO_PLUGIN_KEY);
                    }
                });
            }
        });
    }

    fn merge_configs(&self, files: Vec<&ProtoConfigFile>) -> Result<ProtoConfig, ProtoConfigError> {
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
