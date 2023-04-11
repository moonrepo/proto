use crate::GoLanguage;
use proto_core::{async_trait, get_home_dir, Describable, Executable, Installable, ProtoError};
use std::{
    env,
    path::{Path, PathBuf},
};

#[cfg(target_os = "windows")]
pub fn get_bin_name<T: AsRef<str>>(name: T) -> String {
    format!("bin/{}.exe", name.as_ref())
}

#[cfg(not(target_os = "windows"))]
pub fn get_bin_name<T: AsRef<str>>(name: T) -> String {
    format!("bin/{}", name.as_ref())
}

#[async_trait]
impl Executable<'_> for GoLanguage {
    async fn find_bin_path(&mut self) -> Result<(), ProtoError> {
        let bin_path = self
            .get_install_dir()?
            .join(get_bin_name(self.get_bin_name()));

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
        if let Ok(root) = env::var("GOBIN") {
            return Ok(PathBuf::from(root));
        }

        let root = if let Ok(root) = env::var("GOROOT") {
            PathBuf::from(root)
        } else if let Ok(root) = env::var("GOPATH") {
            PathBuf::from(root)
        } else {
            get_home_dir()?.join("go")
        };

        Ok(root.join("bin"))
    }
}
