use crate::WasmPlugin;
use proto_core::{
    async_trait, get_home_dir, get_root, Describable, Executable, Installable, ProtoError,
};
use proto_pdk::{ExecuteParamsInput, ExecuteParamsOutput};
use std::env;
use std::path::{Path, PathBuf};

#[async_trait]
impl Executable<'_> for WasmPlugin {
    async fn find_bin_path(&mut self) -> Result<(), ProtoError> {
        let install_dir = self.get_install_dir()?;
        let mut bin_path = None;

        if self.has_func("find_bins") {
            let execute_params: ExecuteParamsOutput = self.cache_func_with(
                "find_bins",
                ExecuteParamsInput {
                    env: self.get_environment(),
                    tool_dir: self.to_wasi_virtual_path(&install_dir),
                },
            )?;

            if let Some(bin) = &execute_params.bin_path {
                bin_path = Some(install_dir.join(bin));
            }
        }

        if bin_path.is_none() {
            let install_params = self.get_install_params()?;

            bin_path = Some(if let Some(bin) = &install_params.bin_path {
                install_dir.join(bin)
            } else {
                install_dir.join(self.get_id())
            });
        }

        if bin_path.as_ref().is_some_and(|p| p.exists()) {
            self.bin_path = bin_path;
        } else {
            return Err(ProtoError::ExecuteMissingBin(
                self.get_name(),
                bin_path.unwrap(),
            ));
        }

        Ok(())
    }

    fn get_bin_path(&self) -> Result<&Path, ProtoError> {
        match self.bin_path.as_ref() {
            Some(bin) => Ok(bin),
            None => Err(ProtoError::MissingTool(self.get_name())),
        }
    }

    fn get_globals_bin_dir(&self) -> Result<Option<PathBuf>, ProtoError> {
        if !self.has_func("find_bins") {
            return Ok(None);
        }

        let home_dir = get_home_dir()?;
        let root_dir = get_root()?;
        let tool_dir = self.get_install_dir()?;
        let env_var_pattern = regex::Regex::new(r"\$([A-Z0-9_]+)").unwrap();

        let params: ExecuteParamsOutput = self.cache_func_with(
            "find_bins",
            ExecuteParamsInput {
                env: self.get_environment(),
                tool_dir: self.to_wasi_virtual_path(&tool_dir),
            },
        )?;

        'outer: for dir_lookup in params.globals_lookup_dirs {
            let mut dir = dir_lookup.clone();

            // If a lookup contains an env var, find and replace it.
            // If the var is not defined or is empty, skip this lookup.
            for cap in env_var_pattern.captures_iter(&dir_lookup) {
                let var = cap.get(0).unwrap().as_str();

                let var_value = match var.as_ref() {
                    "$HOME" => home_dir.to_string_lossy().to_string(),
                    "$PROTO_ROOT" => root_dir.to_string_lossy().to_string(),
                    "$TOOL_DIR" => tool_dir.to_string_lossy().to_string(),
                    _ => env::var(cap.get(1).unwrap().as_str()).unwrap_or_default(),
                };

                if var_value.is_empty() {
                    continue 'outer;
                }

                dir = dir.replace(var, &var_value);
            }

            let dir_path = if let Some(dir_suffix) = dir.strip_prefix('~') {
                home_dir.join(dir_suffix)
            } else {
                PathBuf::from(dir)
            };

            if dir_path.exists() {
                return Ok(Some(dir_path));
            }
        }

        Ok(None)
    }
}
