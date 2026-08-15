use crate::config::*;
use crate::config_error::ProtoConfigError;
use crate::id::Id;
use crate::lockfile::*;
use crate::tool_context::ToolContext;
use once_cell::sync::OnceCell;
use schematic::{Config, PartialConfig};
use serde::Serialize;
use starbase_utils::fs;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, warn};
use version_spec::UnresolvedVersionSpec;

#[derive(Debug, Serialize)]
pub struct ProtoConfigFile {
    pub exists: bool,
    pub path: PathBuf,
    pub config: PartialProtoConfig,

    /// Whether the sibling lockfile of this config has been enabled
    /// and loaded. Each config owns its own lockfile: `.prototools`
    /// is locked to `.protolock`, and `.prototools.<env>` is locked
    /// to `.protolock.<env>`.
    pub locked: bool,
}

#[derive(Debug, Serialize)]
pub struct ProtoDirEntry {
    pub path: PathBuf,
    pub location: PinLocation,
    // Configs are ordered by precedence, so the environment
    // config (when enabled) comes before the base config
    pub configs: Vec<ProtoConfigFile>,
}

impl ProtoDirEntry {
    /// Gather all version specs defined in configs within this directory,
    /// including environment scoped configs (`.prototools.{env}`) that are
    /// not currently active, as all configs in the directory share the same
    /// lockfile, and their specs must be considered used.
    ///
    /// Returns `None` when a config could not be loaded, as the full set of
    /// specs cannot be reliably determined.
    pub fn gather_config_specs(
        &self,
    ) -> Option<BTreeMap<ToolContext, BTreeSet<UnresolvedVersionSpec>>> {
        let mut specs: BTreeMap<ToolContext, BTreeSet<UnresolvedVersionSpec>> = BTreeMap::default();

        fn collect(
            specs: &mut BTreeMap<ToolContext, BTreeSet<UnresolvedVersionSpec>>,
            config: &PartialProtoConfig,
        ) {
            if let Some(versions) = &config.versions {
                for (context, spec) in versions {
                    specs
                        .entry(context.to_owned())
                        .or_default()
                        .insert(spec.req.clone());
                }
            }
        }

        // Configs already loaded, for the active environment mode
        for file in &self.configs {
            collect(&mut specs, &file.config);
        }

        // Environment scoped configs that are not active
        let env_prefix = format!("{PROTO_CONFIG_NAME}.");

        let dir = match fs::read_dir(&self.path) {
            Ok(dir) => dir,
            Err(error) => {
                warn!(
                    dir = ?self.path,
                    "Failed to detect environment scoped configs: {error}",
                );

                return None;
            }
        };

        for dir_entry in dir {
            let path = dir_entry.path();

            if !path.is_file()
                || !path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&env_prefix))
                || self.configs.iter().any(|file| file.path == path)
            {
                continue;
            }

            match ProtoConfig::load(&path, false) {
                Ok(config) => {
                    collect(&mut specs, &config);
                }
                Err(error) => {
                    warn!(
                        file = ?path,
                        "Failed to load environment scoped config: {error}",
                    );

                    return None;
                }
            };
        }

        Some(specs)
    }
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

    // Lockfiles keyed by the path of the config
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
            let is_end = end_dir.is_some_and(|end| end == dir);
            let location = if is_end {
                PinLocation::User
            } else {
                PinLocation::Local
            };

            // Config and lockfile paths, ordered by precedence
            let mut paths = vec![];

            if let Some(env) = env_mode {
                paths.push((
                    dir.join(format!("{PROTO_CONFIG_NAME}.{env}")),
                    dir.join(format!("{PROTO_LOCK_NAME}.{env}")),
                ));
            }

            paths.push((dir.join(PROTO_CONFIG_NAME), dir.join(PROTO_LOCK_NAME)));

            let mut configs = Vec::with_capacity(paths.len());
            let mut inherited_lockfile = None;

            // Load in reverse so that the base config comes first,
            // as environment configs inherit settings from it
            for (config_path, lock_path) in paths.into_iter().rev() {
                let mut file = ProtoConfigFile {
                    config: ProtoConfig::load(&config_path, false)?,
                    exists: config_path.exists(),
                    path: config_path,
                    locked: false,
                };

                // A lockfile is scoped to the config in which it was enabled,
                // and is never enabled for user or global configs. Since an
                // environment config layers on top of the base config in the
                // same directory, it inherits the setting when not set
                let lockfile = file
                    .config
                    .settings
                    .as_ref()
                    .and_then(|settings| settings.lockfile)
                    .or(inherited_lockfile);

                inherited_lockfile = lockfile;

                if location == PinLocation::Local && file.exists && lockfile.unwrap_or(false) {
                    locks.insert(
                        file.path.clone(),
                        Arc::new(RwLock::new(ProtoLock::load(&lock_path)?)),
                    );

                    file.locked = true;
                } else if lock_path.exists() {
                    fs::remove_file(lock_path)?;
                }

                configs.push(file);
            }

            // Restore precedence order
            configs.reverse();

            entries.push(ProtoDirEntry {
                path: dir.to_path_buf(),
                location,
                configs,
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

    /// Return the config whose lockfile applies to the provided tool, if
    /// there is one. A lockfile is scoped to the config in which it was
    /// enabled: it only applies to tools with versions defined in that config,
    /// or ad-hoc installs within that config's directory scope, and never to
    /// tools defined in other (nested, sibling, global, or user) configs.
    pub fn get_locked_config(&self, context: &ToolContext) -> Option<&ProtoConfigFile> {
        // The closest config that defines a version for the tool
        // owns its lock records
        for file in self.entries.iter().flat_map(|dir| &dir.configs) {
            if file.exists
                && file
                    .config
                    .versions
                    .as_ref()
                    .is_some_and(|versions| versions.contains_key(context))
            {
                return file.locked.then_some(file);
            }
        }

        // When not defined in any config, treat the tool as an ad-hoc
        // install owned by the closest directory with a config. Since
        // ad-hoc installs are not environment specific, prefer the base
        // config over the environment config in that directory
        for entry in &self.entries {
            if entry.location == PinLocation::Local
                && let Some(file) = entry.configs.iter().rev().find(|file| file.exists)
            {
                return file.locked.then_some(file);
            }
        }

        None
    }

    /// Return the lockfile that was enabled by the config at the provided path.
    pub fn get_lock(
        &self,
        config_path: &Path,
    ) -> Result<Option<RwLockReadGuard<'_, ProtoLock>>, ProtoConfigError> {
        self.locks
            .get(config_path)
            .map(|lock| {
                lock.read()
                    .map_err(|_| ProtoConfigError::FailedLockfileLock)
            })
            .transpose()
    }

    /// Return the lockfile that was enabled by the config at the provided path,
    /// for mutation.
    pub fn get_lock_mut(
        &self,
        config_path: &Path,
    ) -> Result<Option<RwLockWriteGuard<'_, ProtoLock>>, ProtoConfigError> {
        self.locks
            .get(config_path)
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
