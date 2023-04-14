use crate::SchemaPlugin;
use proto_core::{async_trait, get_home_dir, Describable, Executable, Installable, ProtoError};
use std::{
    env::{self, consts},
    path::{Path, PathBuf},
};

#[async_trait]
impl Executable<'_> for SchemaPlugin {
    async fn find_bin_path(&mut self) -> Result<(), ProtoError> {
        let bin = if let Some(bin_paths) = &self.schema.execute.bin_path {
            bin_paths
                .get(consts::OS)
                .map(|b| b.to_owned())
                .unwrap_or(self.get_bin_name().to_owned())
        } else if cfg!(windows) {
            format!("{}.exe", self.get_bin_name())
        } else {
            self.get_bin_name().to_owned()
        };

        let bin_path = self.get_install_dir()?.join(bin);

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
        let env_var_pattern = regex::Regex::new("$([A-Z0-9_]+)").unwrap();

        for dir in &self.schema.execute.globals_dir {
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

        Ok(home_dir
            .join(format!(".{}", self.get_bin_name()))
            .join("bin"))
    }
}
