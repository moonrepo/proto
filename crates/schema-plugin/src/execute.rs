use crate::SchemaPlugin;
use proto_core::{
    async_trait, get_bin_name, get_home_dir, Describable, Executable, Installable, ProtoError,
};
use std::{
    env,
    path::{Path, PathBuf},
};

#[async_trait]
impl Executable<'_> for SchemaPlugin {
    async fn find_bin_path(&mut self) -> Result<(), ProtoError> {
        let mut bin = None;

        if let Ok(platform) = self.get_platform() {
            if let Some(bin_path) = &platform.bin_path {
                bin = Some(bin_path.to_owned());
            }
        }

        if bin.is_none() {
            bin = Some(get_bin_name(self.get_id()));
        }

        let bin_path = self.get_install_dir()?.join(bin.unwrap());

        if bin_path.exists() {
            self.bin_path = Some(bin_path);
        } else {
            return Err(ProtoError::ExecuteMissingBin(self.get_name(), bin_path));
        }

        Ok(())
    }

    fn get_bin_path(&self) -> Result<&Path, ProtoError> {
        match self.bin_path.as_ref() {
            Some(bin) => Ok(bin),
            None => Err(ProtoError::MissingTool(self.get_name())),
        }
    }

    fn get_globals_bin_dir(&self) -> Result<PathBuf, ProtoError> {
        let home_dir = get_home_dir()?;
        let env_var_pattern = regex::Regex::new(r"\$([A-Z0-9_]+)").unwrap();

        for dir in &self.schema.install.globals_dir {
            let dir = env_var_pattern.replace_all(dir, |cap: &regex::Captures| {
                env::var(cap.get(1).unwrap().as_str()).unwrap_or_default()
            });

            let dir_path = if let Some(dir_suffix) = dir.strip_prefix('~') {
                home_dir.join(dir_suffix)
            } else {
                PathBuf::from(dir.to_string())
            };

            if dir_path.exists() {
                return Ok(dir_path);
            }
        }

        Ok(home_dir.join(format!(".{}", self.get_id())).join("bin"))
    }
}
