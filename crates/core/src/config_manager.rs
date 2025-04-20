use crate::config::*;
use crate::lockfile::*;
use once_cell::sync::OnceCell;
use schematic::{Config, PartialConfig};
use serde::Serialize;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Serialize)]
pub struct ProtoFiles {
    pub exists: bool,
    pub location: PinLocation,
    pub lockfile: Option<ProtoLockfile>,
    pub lockfile_path: Option<PathBuf>,
    pub config: PartialProtoConfig,
    pub config_path: PathBuf,
}

#[derive(Debug)]
pub struct ProtoConfigManager {
    // Paths are sorted from current working directory,
    // up until the root or user directory, whichever is first.
    // The special `~/.proto/.prototools` config is always
    // loaded last, and is the last entry in the list.
    // For directories without a config, we still insert
    // an empty entry. This helps with traversal logic.
    pub files: Vec<ProtoFiles>,

    all: OnceCell<(ProtoConfig, ProtoLockfile)>,
    all_no_global: OnceCell<(ProtoConfig, ProtoLockfile)>,
    global: OnceCell<(ProtoConfig, ProtoLockfile)>,
    local: OnceCell<(ProtoConfig, ProtoLockfile)>,
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

                files.push(ProtoFiles {
                    exists: env_path.exists(),
                    config: ProtoConfig::load(&env_path)?,
                    config_path: env_path,
                    location,
                    lockfile: None,
                    lockfile_path: None,
                });
            }

            let config_path = dir.join(PROTO_CONFIG_NAME);
            let lockfile_path = dir.join(PROTO_LOCKFILE_NAME);

            files.push(ProtoFiles {
                config: ProtoConfig::load(&config_path)?,
                exists: config_path.exists(),
                location,
                lockfile: if lockfile_path.exists() {
                    Some(ProtoLockfile::load(&lockfile_path)?)
                } else {
                    None
                },
                lockfile_path: Some(lockfile_path),
                config_path,
            });

            if is_end {
                break;
            }

            current_dir = dir.parent();
        }

        Ok(Self {
            files,
            all: OnceCell::new(),
            all_no_global: OnceCell::new(),
            global: OnceCell::new(),
            local: OnceCell::new(),
        })
    }

    pub fn get_global_config(&self) -> miette::Result<&ProtoConfig> {
        Ok(&self.get_global()?.0)
    }

    pub fn get_local_config(&self, cwd: &Path) -> miette::Result<&ProtoConfig> {
        Ok(&self.get_local(cwd)?.0)
    }

    pub fn get_local_lockfile(&self, cwd: &Path) -> miette::Result<&ProtoLockfile> {
        Ok(&self.get_local(cwd)?.1)
    }

    pub fn get_merged_config(&self) -> miette::Result<&ProtoConfig> {
        Ok(&self.get_all()?.0)
    }

    pub fn get_merged_lockfile(&self) -> miette::Result<&ProtoLockfile> {
        Ok(&self.get_all()?.1)
    }

    pub fn get_merged_config_without_global(&self) -> miette::Result<&ProtoConfig> {
        Ok(&self.get_all_without_global()?.0)
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

    fn get_global(&self) -> miette::Result<&(ProtoConfig, ProtoLockfile)> {
        self.global.get_or_try_init(|| {
            debug!("Loading global config only");

            self.merge_files(
                self.files
                    .iter()
                    .filter(|file| file.location == PinLocation::Global)
                    .collect(),
            )
        })
    }

    fn get_local(&self, cwd: &Path) -> miette::Result<&(ProtoConfig, ProtoLockfile)> {
        self.local.get_or_try_init(|| {
            debug!("Loading local config/lockfile only");

            self.merge_files(
                self.files
                    .iter()
                    .filter(|file| file.config_path.parent().is_some_and(|dir| dir == cwd))
                    .collect(),
            )
        })
    }

    fn get_all(&self) -> miette::Result<&(ProtoConfig, ProtoLockfile)> {
        self.all.get_or_try_init(|| {
            debug!("Merging loaded configs/lockfiles with global");

            self.merge_files(self.files.iter().collect())
        })
    }

    fn get_all_without_global(&self) -> miette::Result<&(ProtoConfig, ProtoLockfile)> {
        self.all_no_global.get_or_try_init(|| {
            debug!("Merging loaded configs/lockfiles without global");

            self.merge_files(
                self.files
                    .iter()
                    .filter(|file| file.location != PinLocation::Global)
                    .collect(),
            )
        })
    }

    fn merge_files(&self, files: Vec<&ProtoFiles>) -> miette::Result<(ProtoConfig, ProtoLockfile)> {
        let mut lockfile = ProtoLockfile::default();
        let mut lockfile_count = 0;
        let mut config = PartialProtoConfig::default();
        let mut config_count = 0;
        let context = &();

        for file in files.iter().rev() {
            if file.exists {
                config.merge(context, file.config.to_owned())?;
                config_count += 1;

                // Lockfiles cannot exist in the global location
                if file.location != PinLocation::Global {
                    if let Some(inner_lockfile) = &file.lockfile {
                        lockfile.tools.extend(inner_lockfile.tools.clone());
                        lockfile_count += 1;
                    }
                }
            }
        }

        let mut config = ProtoConfig::from_partial(config.finalize(context)?);
        config.inherit_builtin_plugins();
        config.setup_env_vars();

        debug!(
            "Merged {} configs and {} lockfiles",
            config_count, lockfile_count
        );

        Ok((config, lockfile))
    }
}
