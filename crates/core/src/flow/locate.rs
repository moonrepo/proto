pub use super::locate_error::ProtoLocateError;
use crate::helpers::ENV_VAR;
use crate::layout::BinManager;
use crate::tool::Tool;
use proto_pdk_api::{
    ExecutableConfig, LocateExecutablesInput, LocateExecutablesOutput, PluginFunction,
};
use proto_shim::{get_exe_file_name, get_shim_file_name};
use serde::Serialize;
use starbase_utils::{fs, path};
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};
use version_spec::VersionSpec;

// Executable = File within the tool's install directory
// Binary/shim = File within proto's store directories

#[derive(Debug, Default, Serialize)]
pub struct ExecutableLocation {
    pub config: ExecutableConfig,
    pub name: String,
    pub path: PathBuf,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<VersionSpec>,
}

impl Tool {
    pub(crate) async fn call_locate_executables(
        &self,
    ) -> Result<LocateExecutablesOutput, ProtoLocateError> {
        Ok(self
            .plugin
            .cache_func_with(
                PluginFunction::LocateExecutables,
                LocateExecutablesInput {
                    context: self.create_plugin_context(),
                    install_dir: self.to_virtual_path(self.get_product_dir()),
                },
            )
            .await?)
    }

    /// Return location information for the primary executable within the tool directory.
    pub async fn resolve_primary_exe_location(
        &self,
    ) -> Result<Option<ExecutableLocation>, ProtoLocateError> {
        let output = self.call_locate_executables().await?;
        let mut primary = None;

        for (name, config) in output.exes {
            let Some(exe_path) = &config.exe_path else {
                continue;
            };

            let path = self
                .get_product_dir()
                .join(path::normalize_separators(exe_path));

            if config.update_perms && path.exists() {
                fs::update_perms(&path, None)?;
            }

            if config.primary {
                primary = Some(ExecutableLocation {
                    path,
                    name,
                    config,
                    version: None,
                });
            }
        }

        Ok(primary)
    }

    /// Return location information for all secondary executables within the tool directory.
    pub async fn resolve_secondary_exe_locations(
        &self,
    ) -> Result<Vec<ExecutableLocation>, ProtoLocateError> {
        let output = self.call_locate_executables().await?;
        let mut locations = vec![];

        for (name, config) in output.exes {
            if config.primary {
                continue;
            }

            if let Some(exe_path) = &config.exe_path {
                locations.push(ExecutableLocation {
                    path: self
                        .get_product_dir()
                        .join(path::normalize_separators(exe_path)),
                    name,
                    config,
                    version: None,
                });
            }
        }

        Ok(locations)
    }

    /// Return a list of all binaries that get created in `~/.proto/bin`.
    /// The list will contain the executable config, and an absolute path
    /// to the binaries final location.
    pub async fn resolve_bin_locations(
        &mut self,
        include_all_versions: bool,
    ) -> Result<Vec<ExecutableLocation>, ProtoLocateError> {
        self.resolve_bin_locations_with_manager(
            BinManager::from_manifest(&self.inventory.manifest),
            include_all_versions,
        )
        .await
    }

    pub async fn resolve_bin_locations_with_manager(
        &mut self,
        bin_manager: BinManager,
        include_all_versions: bool,
    ) -> Result<Vec<ExecutableLocation>, ProtoLocateError> {
        let original_version = self.get_resolved_version();
        let mut locations = vec![];

        let versions = if include_all_versions {
            bin_manager.get_buckets()
        } else {
            bin_manager.get_buckets_focused_to_version(&original_version)
        };

        // Loop through each version, extract the locations,
        // and append it to the master list
        for (bucket_version, resolved_version) in versions {
            // Locate the executables for this specific version,
            // as the logic in how they are located may have changed
            // between versions, and we simply can't rely on the
            // latest version being completely backwards compatible
            self.set_version(resolved_version.to_owned());

            let output: LocateExecutablesOutput = self
                .plugin
                .cache_func_with(
                    PluginFunction::LocateExecutables,
                    LocateExecutablesInput {
                        context: self.create_plugin_context(),
                        install_dir: self.to_virtual_path(self.get_product_dir()),
                    },
                )
                .await?;

            let mut add = |name: String, config: ExecutableConfig| {
                if !config.no_bin
                    && config
                        .exe_link_path
                        .as_ref()
                        .or(config.exe_path.as_ref())
                        .is_some()
                {
                    let versioned_name = if *bucket_version == "*" {
                        name.clone()
                    } else {
                        format!("{name}-{bucket_version}")
                    };

                    locations.push(ExecutableLocation {
                        path: self
                            .proto
                            .store
                            .bin_dir
                            .join(get_exe_file_name(&versioned_name)),
                        name: versioned_name,
                        config: config.clone(),
                        version: Some((*resolved_version).to_owned()),
                    });
                }
            };

            if !output.exes.is_empty() {
                for (name, config) in output.exes {
                    add(name, config);
                }
            }
        }

        // self.backend = original_backend;
        self.set_version(original_version);

        locations.sort_by(|a, d| a.name.cmp(&d.name));

        Ok(locations)
    }

    /// Return a list of all shims that get created in `~/.proto/shims`.
    /// The list will contain the executable config, and an absolute path
    /// to the shims final location.
    pub async fn resolve_shim_locations(
        &self,
    ) -> Result<Vec<ExecutableLocation>, ProtoLocateError> {
        let output = self.call_locate_executables().await?;
        let mut locations = vec![];

        let mut add = |name: String, config: ExecutableConfig| {
            if !config.no_shim {
                locations.push(ExecutableLocation {
                    path: self.proto.store.shims_dir.join(get_shim_file_name(&name)),
                    name,
                    config,
                    version: None,
                });
            }
        };

        if !output.exes.is_empty() {
            for (name, config) in output.exes {
                add(name, config);
            }
        }

        Ok(locations)
    }

    /// Return an absolute path to the primary executable file, after it has been located.
    pub fn get_exe_file(&self) -> Option<&Path> {
        self.exe_file.as_deref()
    }

    /// Locate the primary executable from the tool directory.
    #[instrument(skip_all)]
    pub async fn locate_exe_file(&mut self) -> Result<PathBuf, ProtoLocateError> {
        if self.exe_file.is_some() {
            return Ok(self.exe_file.clone().unwrap());
        }

        debug!(
            tool = self.context.as_str(),
            "Locating primary executable for tool"
        );

        let exe_file = if let Some(location) = self.resolve_primary_exe_location().await? {
            location.path
        } else {
            self.get_product_dir()
                .join(path::exe_name(path::encode_component(self.get_file_name())))
        };

        if exe_file.exists() {
            debug!(tool = self.context.as_str(), exe_path = ?exe_file, "Found an executable");

            if self.cache_internal {
                self.exe_file = Some(exe_file.clone());
            }

            return Ok(exe_file);
        }

        Err(ProtoLocateError::MissingToolExecutable {
            tool: self.get_name().to_owned(),
            path: exe_file,
        })
    }

    /// Return an absolute path to the primary executables directory (first in the list),
    /// after it has been located.
    pub fn get_exes_dir(&self) -> Option<&Path> {
        self.exes_dirs.first().map(|dir| dir.as_ref())
    }

    /// Return an absolute path to all executable directories, after they have been located.
    pub fn get_exes_dirs(&self) -> &[PathBuf] {
        &self.exes_dirs
    }

    /// Locate the directory that local executables are installed to.
    #[instrument(skip_all)]
    pub async fn locate_exes_dirs(&mut self) -> Result<Vec<PathBuf>, ProtoLocateError> {
        if !self.exes_dirs.is_empty() {
            return Ok(self.exes_dirs.clone());
        }

        let mut dirs = vec![];

        if self
            .plugin
            .has_func(PluginFunction::LocateExecutables)
            .await
        {
            let output = self.call_locate_executables().await?;

            #[allow(deprecated)]
            if let Some(dir) = output.exes_dir {
                dirs.push(self.get_product_dir().join(path::normalize_separators(dir)));
            } else {
                for dir in output.exes_dirs {
                    if dir.to_str().is_some_and(|dir| dir == ".") {
                        dirs.push(self.get_product_dir().to_path_buf());
                    } else {
                        dirs.push(self.get_product_dir().join(path::normalize_separators(dir)));
                    }
                }
            }
        }

        if self.cache_internal {
            self.exes_dirs = dirs.clone();
        }

        Ok(dirs)
    }

    /// Return an absolute path to the globals directory, after it has been located.
    pub fn get_globals_dir(&self) -> Option<&Path> {
        self.globals_dir.as_deref()
    }

    /// Return an absolute path to the globals directory that actually exists
    /// and contains files (executables).
    #[instrument(skip_all)]
    pub async fn locate_globals_dir(&mut self) -> Result<Option<PathBuf>, ProtoLocateError> {
        if self.globals_dir.is_some() {
            return Ok(self.globals_dir.clone());
        }

        let globals_dirs = self.locate_globals_dirs().await?;
        let mut found_dir = None;

        for dir in &globals_dirs {
            if !dir.exists() {
                continue;
            }

            let has_files = fs::read_dir(dir).is_ok_and(|list| {
                !list
                    .into_iter()
                    .filter(|entry| entry.path().is_file())
                    .collect::<Vec<_>>()
                    .is_empty()
            });

            if has_files {
                debug!(tool = self.context.as_str(), dir = ?dir, "Found a usable globals directory");

                found_dir = Some(dir.to_owned());
                break;
            }
        }

        if found_dir.is_none()
            && let Some(dir) = globals_dirs.last()
        {
            debug!(
                tool = self.context.as_str(),
                dir = ?dir,
                "No usable globals directory found, falling back to the last entry",
            );

            found_dir = Some(dir.to_owned());
        }

        // Ensure directory exists as some tools require it
        if let Some(dir) = &found_dir {
            let _ = fs::create_dir_all(dir);
        }

        if self.cache_internal {
            self.globals_dir = found_dir.clone();
        }

        Ok(found_dir)
    }

    /// Return an absolute path to all globals directories, after they have been located.
    pub fn get_globals_dirs(&self) -> &[PathBuf] {
        &self.globals_dirs
    }

    /// Locate the directories that global packages are installed to.
    /// Will expand environment variables, and filter out invalid paths.
    #[instrument(skip_all)]
    pub async fn locate_globals_dirs(&mut self) -> Result<Vec<PathBuf>, ProtoLocateError> {
        if !self.globals_dirs.is_empty() {
            return Ok(self.globals_dirs.clone());
        }

        if !self
            .plugin
            .has_func(PluginFunction::LocateExecutables)
            .await
        {
            return Ok(vec![]);
        }

        debug!(
            tool = self.context.as_str(),
            "Locating globals directories for tool"
        );

        let output = self.call_locate_executables().await?;

        // Set the prefix for simpler caching
        if self.cache_internal {
            self.globals_prefix = output.globals_prefix;
        }

        // Find all possible global directories that packages can be installed to
        let install_dir = self.get_product_dir();
        let mut resolved_dirs = vec![];

        'outer: for dir_lookup in output.globals_lookup_dirs {
            let mut dir = dir_lookup.clone();

            // If a lookup contains an env var, find and replace it.
            // If the var is not defined or is empty, skip this lookup.
            for cap in ENV_VAR.captures_iter(&dir_lookup) {
                let find_by = cap.get(0).unwrap().as_str();

                let replace_with = match find_by {
                    "$CWD" | "$PWD" => self.proto.working_dir.clone(),
                    "$HOME" | "$USERHOME" | "$USERPROFILE" => self.proto.home_dir.clone(),
                    "$PROTO_HOME" | "$PROTO_ROOT" => self.proto.store.dir.clone(),
                    "$TOOL_DIR" => install_dir.to_path_buf(),
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
                self.proto
                    .home_dir
                    .join(path::normalize_separators(dir_suffix))
            } else {
                PathBuf::from(path::normalize_separators(dir))
            };

            // Don't use a set as we need to persist the order!
            if !resolved_dirs.contains(&dir) {
                resolved_dirs.push(dir);
            }
        }

        debug!(
            tool = self.context.as_str(),
            dirs = ?resolved_dirs,
            "Located possible globals directories",
        );

        if self.cache_internal {
            self.globals_dirs = resolved_dirs.clone();
        }

        Ok(resolved_dirs)
    }

    /// Return the globals prefix, after it has been located.
    pub fn get_globals_prefix(&self) -> Option<&str> {
        self.globals_prefix.as_deref()
    }

    /// Return a string that all globals are prefixed with. Will be used for filtering and listing.
    #[instrument(skip_all)]
    pub async fn locate_globals_prefix(&mut self) -> Result<Option<String>, ProtoLocateError> {
        if self.globals_prefix.is_some() {
            return Ok(self.globals_prefix.clone());
        }

        if !self
            .plugin
            .has_func(PluginFunction::LocateExecutables)
            .await
        {
            return Ok(None);
        }

        let output = self.call_locate_executables().await?;
        let prefix = output.globals_prefix;

        if self.cache_internal {
            self.globals_prefix = prefix.clone();
        }

        Ok(prefix)
    }
}
