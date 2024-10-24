use crate::error::ProtoError;
use crate::helpers::ENV_VAR;
use crate::tool::Tool;
use proto_pdk_api::{ExecutableConfig, LocateExecutablesInput, LocateExecutablesOutput};
use proto_shim::{get_exe_file_name, get_shim_file_name};
use serde::Serialize;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};

// Executable = File within the tool's install directory
// Binary/shim = File within proto's store directories

#[derive(Debug, Default, Serialize)]
pub struct ExecutableLocation {
    pub config: ExecutableConfig,
    pub name: String,
    pub path: PathBuf,
}

impl Tool {
    pub(crate) async fn call_locate_executables(&self) -> miette::Result<LocateExecutablesOutput> {
        self.plugin
            .cache_func_with(
                "locate_executables",
                LocateExecutablesInput {
                    context: self.create_context(),
                },
            )
            .await
    }

    /// Return location information for the primary executable within the tool directory.
    pub async fn resolve_primary_exe_location(&self) -> miette::Result<Option<ExecutableLocation>> {
        let output = self.call_locate_executables().await?;

        for (name, config) in output.exes {
            if config.primary {
                if let Some(exe_path) = &config.exe_path {
                    return Ok(Some(ExecutableLocation {
                        path: self.get_product_dir().join(exe_path),
                        name,
                        config,
                    }));
                }
            }
        }

        #[allow(deprecated)]
        if let Some(mut primary) = output.primary {
            if let Some(exe_path) = &primary.exe_path {
                primary.primary = true;

                return Ok(Some(ExecutableLocation {
                    path: self.get_product_dir().join(exe_path),
                    name: self.id.to_string(),
                    config: primary,
                }));
            }
        }

        Ok(None)
    }

    /// Return location information for all secondary executables within the tool directory.
    pub async fn resolve_secondary_exe_locations(&self) -> miette::Result<Vec<ExecutableLocation>> {
        let output = self.call_locate_executables().await?;
        let mut locations = vec![];

        for (name, config) in output.exes {
            if config.primary {
                continue;
            }

            if let Some(exe_path) = &config.exe_path {
                locations.push(ExecutableLocation {
                    path: self.get_product_dir().join(exe_path),
                    name,
                    config,
                });
            }
        }

        if locations.is_empty() {
            #[allow(deprecated)]
            for (name, secondary) in output.secondary {
                if let Some(exe_path) = &secondary.exe_path {
                    locations.push(ExecutableLocation {
                        path: self.get_product_dir().join(exe_path),
                        name,
                        config: secondary,
                    });
                }
            }
        }

        Ok(locations)
    }

    /// Return a list of all binaries that get created in `~/.proto/bin`.
    /// The list will contain the executable config, and an absolute path
    /// to the binaries final location.
    pub async fn resolve_bin_locations(&self) -> miette::Result<Vec<ExecutableLocation>> {
        let output = self.call_locate_executables().await?;
        let mut locations = vec![];

        let mut add = |name: String, config: ExecutableConfig| {
            if !config.no_bin
                && config
                    .exe_link_path
                    .as_ref()
                    .or(config.exe_path.as_ref())
                    .is_some()
            {
                locations.push(ExecutableLocation {
                    path: self.proto.store.bin_dir.join(get_exe_file_name(&name)),
                    name,
                    config,
                });
            }
        };

        if output.exes.is_empty() {
            #[allow(deprecated)]
            if let Some(mut primary) = output.primary {
                primary.primary = true;

                add(self.id.to_string(), primary);
            }

            #[allow(deprecated)]
            for (name, secondary) in output.secondary {
                add(name, secondary);
            }
        } else {
            for (name, config) in output.exes {
                add(name, config);
            }
        }

        Ok(locations)
    }

    /// Return a list of all shims that get created in `~/.proto/shims`.
    /// The list will contain the executable config, and an absolute path
    /// to the shims final location.
    pub async fn resolve_shim_locations(&self) -> miette::Result<Vec<ExecutableLocation>> {
        let output = self.call_locate_executables().await?;
        let mut locations = vec![];

        let mut add = |name: String, config: ExecutableConfig| {
            if !config.no_shim {
                locations.push(ExecutableLocation {
                    path: self.proto.store.shims_dir.join(get_shim_file_name(&name)),
                    name,
                    config,
                });
            }
        };

        if output.exes.is_empty() {
            #[allow(deprecated)]
            if let Some(mut primary) = output.primary {
                primary.primary = true;

                add(self.id.to_string(), primary);
            }

            #[allow(deprecated)]
            for (name, secondary) in output.secondary {
                add(name, secondary);
            }
        } else {
            for (name, config) in output.exes {
                add(name, config);
            }
        }

        Ok(locations)
    }

    /// Locate the primary executable from the tool directory.
    #[instrument(skip_all)]
    pub async fn locate_exe_file(&mut self) -> miette::Result<PathBuf> {
        if self.exe_file.is_some() {
            return Ok(self.exe_file.clone().unwrap());
        }

        debug!(
            tool = self.id.as_str(),
            "Locating primary executable for tool"
        );

        let exe_file = if let Some(location) = self.resolve_primary_exe_location().await? {
            location.path
        } else {
            self.get_product_dir().join(self.id.as_str())
        };

        if exe_file.exists() {
            debug!(tool = self.id.as_str(), exe_path = ?exe_file, "Found an executable");

            self.exe_file = Some(exe_file.clone());

            return Ok(exe_file);
        }

        Err(ProtoError::MissingToolExecutable {
            tool: self.get_name().to_owned(),
            path: exe_file,
        }
        .into())
    }

    /// Return an absolute path to the executables directory, after it has been located.
    pub fn get_exes_dir(&self) -> Option<&Path> {
        self.exes_dir.as_deref()
    }

    /// Locate the directory that local executables are installed to.
    #[instrument(skip_all)]
    pub async fn locate_exes_dir(&mut self) -> miette::Result<Option<PathBuf>> {
        if self.exes_dir.is_none() {
            if !self.plugin.has_func("locate_executables").await {
                return Ok(None);
            }

            let output = self.call_locate_executables().await?;

            if let Some(exes_dir) = output.exes_dir {
                self.exes_dir = Some(self.get_product_dir().join(exes_dir));
            }
        }

        Ok(self.exes_dir.clone())
    }

    /// Return an absolute path to the globals directory, after it has been located.
    pub fn get_globals_dir(&self) -> Option<&Path> {
        self.globals_dir.as_deref()
    }

    /// Return an absolute path to the globals directory that actually exists
    /// and contains files (binaries).
    #[instrument(skip_all)]
    pub async fn locate_globals_dir(&mut self) -> miette::Result<Option<PathBuf>> {
        if self.globals_dir.is_none() {
            let globals_dirs = self.locate_globals_dirs().await?;
            let lookup_count = globals_dirs.len() - 1;

            for (index, dir) in globals_dirs.into_iter().enumerate() {
                if !dir.exists() {
                    continue;
                }

                let has_files = fs::read_dir(&dir).is_ok_and(|list| {
                    !list
                        .into_iter()
                        .filter(|entry| entry.path().is_file())
                        .collect::<Vec<_>>()
                        .is_empty()
                });

                if has_files {
                    debug!(tool = self.id.as_str(), dir = ?dir, "Found a usable globals directory");

                    self.globals_dir = Some(dir);
                    break;
                }

                if index == lookup_count {
                    debug!(
                        tool = self.id.as_str(),
                        dir = ?dir,
                        "No usable globals directory found, falling back to the last entry",
                    );

                    self.globals_dir = Some(dir);
                    break;
                }
            }
        }

        Ok(self.globals_dir.clone())
    }

    /// Return an absolute path to all globals directories, after they have been located.
    pub fn get_globals_dirs(&self) -> &[PathBuf] {
        &self.globals_dirs
    }

    /// Locate the directories that global packages are installed to.
    /// Will expand environment variables, and filter out invalid paths.
    #[instrument(skip_all)]
    pub async fn locate_globals_dirs(&mut self) -> miette::Result<Vec<PathBuf>> {
        if !self.globals_dirs.is_empty() {
            return Ok(self.globals_dirs.clone());
        }

        if !self.plugin.has_func("locate_executables").await {
            return Ok(vec![]);
        }

        debug!(
            tool = self.id.as_str(),
            "Locating globals directories for tool"
        );

        let install_dir = self.get_product_dir();
        let output = self.call_locate_executables().await?;

        // Set the prefix for simpler caching
        self.globals_prefix = output.globals_prefix;

        // Find all possible global directories that packages can be installed to
        let mut resolved_dirs = vec![];

        'outer: for dir_lookup in output.globals_lookup_dirs {
            let mut dir = dir_lookup.clone();

            // If a lookup contains an env var, find and replace it.
            // If the var is not defined or is empty, skip this lookup.
            for cap in ENV_VAR.captures_iter(&dir_lookup) {
                let find_by = cap.get(0).unwrap().as_str();

                let replace_with = match find_by {
                    "$CWD" | "$PWD" => self.proto.cwd.clone(),
                    "$HOME" | "$USERHOME" => self.proto.home.clone(),
                    "$PROTO_HOME" | "$PROTO_ROOT" => self.proto.root.clone(),
                    "$TOOL_DIR" => install_dir.clone(),
                    _ => match env::var_os(cap.get(1).unwrap().as_str()) {
                        Some(value) => PathBuf::from(value),
                        None => {
                            continue 'outer;
                        }
                    },
                };

                if let Some(replacement) = replace_with.to_str() {
                    dir = dir.replace(find_by, replacement);
                } else {
                    continue 'outer;
                }
            }

            let dir = if let Some(dir_suffix) = dir.strip_prefix('~') {
                self.proto.home.join(dir_suffix)
            } else {
                PathBuf::from(dir)
            };

            // Don't use a set as we need to persist the order!
            if !resolved_dirs.contains(&dir) {
                resolved_dirs.push(dir);
            }
        }

        debug!(
            tool = self.id.as_str(),
            dirs = ?resolved_dirs,
            "Located possible globals directories",
        );

        self.globals_dirs = resolved_dirs.clone();

        Ok(resolved_dirs)
    }

    /// Return the globals prefix, after it has been located.
    pub fn get_globals_prefix(&self) -> Option<&str> {
        self.globals_prefix.as_deref()
    }

    /// Return a string that all globals are prefixed with. Will be used for filtering and listing.
    #[instrument(skip_all)]
    pub async fn locate_globals_prefix(&mut self) -> miette::Result<Option<String>> {
        if self.globals_prefix.is_none() {
            if !self.plugin.has_func("locate_executables").await {
                return Ok(None);
            }

            let output = self.call_locate_executables().await?;

            self.globals_prefix = output.globals_prefix;
        }

        Ok(self.globals_prefix.clone())
    }
}
