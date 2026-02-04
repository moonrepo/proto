pub use super::locate_error::ProtoLocateError;
use crate::helpers::ENV_VAR;
use crate::layout::BinManager;
use crate::tool::Tool;
use crate::tool_spec::ToolSpec;
use proto_pdk_api::{
    ExecutableConfig, LocateExecutablesInput, LocateExecutablesOutput, PluginFunction,
};
use proto_shim::{get_exe_file_name, get_shim_file_name};
use serde::Serialize;
use starbase_utils::{fs, path};
use std::env;
use std::path::PathBuf;
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

#[derive(Debug, Default, Serialize)]
pub struct LocatorResponse {
    pub exe_file: PathBuf,
    pub exes_dirs: Vec<PathBuf>,
    pub globals_dir: Option<PathBuf>,
    pub globals_dirs: Vec<PathBuf>,
    pub globals_prefix: Option<String>,
}

/// Locates executables for installed tools.
pub struct Locator<'tool> {
    tool: &'tool Tool,
    spec: &'tool ToolSpec,

    exe_file: Option<PathBuf>,
    exes_dirs: Vec<PathBuf>,
    globals_dir: Option<PathBuf>,
    globals_dirs: Vec<PathBuf>,
    globals_prefix: Option<String>,

    pub product_dir: PathBuf,
}

impl<'tool> Locator<'tool> {
    pub fn new(tool: &'tool Tool, spec: &'tool ToolSpec) -> Self {
        Self {
            product_dir: tool.get_product_dir(spec),
            tool,
            spec,
            exe_file: None,
            exes_dirs: vec![],
            globals_dir: None,
            globals_dirs: vec![],
            globals_prefix: None,
        }
    }

    pub async fn locate(
        tool: &'tool Tool,
        spec: &'tool ToolSpec,
    ) -> Result<LocatorResponse, ProtoLocateError> {
        Self::new(tool, spec).locate_all().await
    }

    /// Locate all applicable executable and global paths.
    pub async fn locate_all(&mut self) -> Result<LocatorResponse, ProtoLocateError> {
        Ok(LocatorResponse {
            globals_dirs: self.locate_globals_dirs().await?,
            globals_dir: self.locate_globals_dir().await?,
            globals_prefix: self.locate_globals_prefix().await?,
            exe_file: self.locate_exe_file().await?,
            exes_dirs: self.locate_exes_dirs().await?,
        })
    }

    pub(crate) async fn call_locate_executables(
        &self,
    ) -> Result<LocateExecutablesOutput, ProtoLocateError> {
        Ok(self
            .tool
            .plugin
            .cache_func_with(
                PluginFunction::LocateExecutables,
                LocateExecutablesInput {
                    context: self.tool.create_plugin_context(self.spec),
                    install_dir: self.tool.to_virtual_path(&self.product_dir),
                },
            )
            .await?)
    }

    /// Return location information for the primary executable within the tool directory.
    pub async fn locate_primary_exe(&self) -> Result<Option<ExecutableLocation>, ProtoLocateError> {
        let output = self.call_locate_executables().await?;
        let mut primary = None;

        for (name, config) in output.exes {
            let Some(exe_path) = &config.exe_path else {
                continue;
            };

            let path = self.product_dir.join(path::normalize_separators(exe_path));

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
    pub async fn locate_secondary_exes(&self) -> Result<Vec<ExecutableLocation>, ProtoLocateError> {
        let output = self.call_locate_executables().await?;
        let mut locations = vec![];

        for (name, config) in output.exes {
            if config.primary {
                continue;
            }

            if let Some(exe_path) = &config.exe_path {
                locations.push(ExecutableLocation {
                    path: self.product_dir.join(path::normalize_separators(exe_path)),
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
    pub async fn locate_bins(
        &self,
        focused_version: Option<&VersionSpec>,
    ) -> Result<Vec<ExecutableLocation>, ProtoLocateError> {
        self.locate_bins_with_manager(
            BinManager::from_manifest(&self.tool.inventory.manifest),
            focused_version,
        )
        .await
    }

    pub async fn locate_bins_with_manager(
        &self,
        bin_manager: BinManager,
        focused_version: Option<&VersionSpec>,
    ) -> Result<Vec<ExecutableLocation>, ProtoLocateError> {
        let mut locations = vec![];
        let versions = match focused_version {
            Some(version) => bin_manager.get_buckets_focused_to_version(version),
            None => bin_manager.get_buckets(),
        };

        // Loop through each version, extract the locations,
        // and append it to the master list
        for (bucket_version, resolved_version) in versions {
            // Locate the executables for this specific version,
            // as the logic in how they are located may have changed
            // between versions, and we simply can't rely on the
            // latest version being completely backwards compatible
            let spec = ToolSpec::new_resolved(resolved_version.to_owned());

            let output: LocateExecutablesOutput = self
                .tool
                .plugin
                .cache_func_with(
                    PluginFunction::LocateExecutables,
                    LocateExecutablesInput {
                        context: self.tool.create_plugin_context(&spec),
                        install_dir: self.tool.to_virtual_path(self.tool.get_product_dir(&spec)),
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
                            .tool
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

        locations.sort_by(|a, d| a.name.cmp(&d.name));

        Ok(locations)
    }

    /// Return a list of all shims that get created in `~/.proto/shims`.
    /// The list will contain the executable config, and an absolute path
    /// to the shims final location.
    pub async fn locate_shims(&self) -> Result<Vec<ExecutableLocation>, ProtoLocateError> {
        let output = self.call_locate_executables().await?;
        let mut locations = vec![];

        let mut add = |name: String, config: ExecutableConfig| {
            if !config.no_shim {
                locations.push(ExecutableLocation {
                    path: self
                        .tool
                        .proto
                        .store
                        .shims_dir
                        .join(get_shim_file_name(&name)),
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
    pub fn get_exe_file(&self) -> Option<&PathBuf> {
        self.exe_file.as_ref()
    }

    /// Locate the primary executable from the tool directory.
    #[instrument(skip_all)]
    pub async fn locate_exe_file(&mut self) -> Result<PathBuf, ProtoLocateError> {
        if let Some(exe) = &self.exe_file {
            return Ok(exe.to_owned());
        }

        debug!(
            tool = self.tool.context.as_str(),
            "Locating primary executable for tool"
        );

        let exe_file = if let Some(location) = self.locate_primary_exe().await? {
            location.path
        } else {
            self.product_dir.join(path::exe_name(path::encode_component(
                self.tool.get_file_name(),
            )))
        };

        if exe_file.exists() {
            debug!(tool = self.tool.context.as_str(), exe_path = ?exe_file, "Found an executable");

            self.exe_file = Some(exe_file.clone());

            return Ok(exe_file);
        }

        Err(ProtoLocateError::MissingToolExecutable {
            tool: self.tool.get_name().to_owned(),
            path: exe_file,
        })
    }

    /// Return an absolute path to the primary executables directory (first in the list),
    /// after it has been located.
    pub fn get_exes_dir(&self) -> Option<&PathBuf> {
        self.exes_dirs.first()
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
            .tool
            .plugin
            .has_func(PluginFunction::LocateExecutables)
            .await
        {
            let output = self.call_locate_executables().await?;

            #[allow(deprecated)]
            if let Some(dir) = output.exes_dir {
                dirs.push(self.product_dir.join(path::normalize_separators(dir)));
            } else {
                for dir in output.exes_dirs {
                    if dir.to_str().is_some_and(|dir| dir == ".") {
                        dirs.push(self.product_dir.clone());
                    } else {
                        dirs.push(self.product_dir.join(path::normalize_separators(dir)));
                    }
                }
            }
        }

        self.exes_dirs = dirs.clone();

        Ok(dirs)
    }

    /// Return an absolute path to the globals directory, after it has been located.
    pub fn get_globals_dir(&self) -> Option<&PathBuf> {
        self.globals_dir.as_ref()
    }

    /// Return an absolute path to the globals directory that actually exists
    /// and contains files (executables).
    #[instrument(skip_all)]
    pub async fn locate_globals_dir(&mut self) -> Result<Option<PathBuf>, ProtoLocateError> {
        if let Some(dir) = &self.globals_dir {
            return Ok(Some(dir.to_owned()));
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
                debug!(tool = self.tool.context.as_str(), dir = ?dir, "Found a usable globals directory");

                found_dir = Some(dir.to_owned());
                break;
            }
        }

        if found_dir.is_none()
            && let Some(dir) = globals_dirs.last()
        {
            debug!(
                tool = self.tool.context.as_str(),
                dir = ?dir,
                "No usable globals directory found, falling back to the last entry",
            );

            found_dir = Some(dir.to_owned());
        }

        // Ensure directory exists as some tools require it
        if let Some(dir) = &found_dir {
            let _ = fs::create_dir_all(dir);
        }

        self.globals_dir = found_dir.clone();

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
            .tool
            .plugin
            .has_func(PluginFunction::LocateExecutables)
            .await
        {
            return Ok(vec![]);
        }

        debug!(
            tool = self.tool.context.as_str(),
            "Locating globals directories for tool"
        );

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
                    "$CWD" | "$PWD" => self.tool.proto.working_dir.clone(),
                    "$HOME" | "$USERHOME" | "$USERPROFILE" => self.tool.proto.home_dir.clone(),
                    "$PROTO_HOME" | "$PROTO_ROOT" => self.tool.proto.store.dir.clone(),
                    "$TOOL_DIR" => self.product_dir.clone(),
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
                self.tool
                    .proto
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
            tool = self.tool.context.as_str(),
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
    pub async fn locate_globals_prefix(&mut self) -> Result<Option<String>, ProtoLocateError> {
        if let Some(prefix) = &self.globals_prefix {
            return Ok(Some(prefix.to_owned()));
        }

        if !self
            .tool
            .plugin
            .has_func(PluginFunction::LocateExecutables)
            .await
        {
            return Ok(None);
        }

        let output = self.call_locate_executables().await?;
        let prefix = output.globals_prefix;

        self.globals_prefix = prefix.clone();

        Ok(prefix)
    }
}
