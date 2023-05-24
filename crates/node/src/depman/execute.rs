use crate::depman::{NodeDependencyManager, NodeDependencyManagerType};
use clean_path::Clean;
use proto_core::{async_trait, Describable, Executable, Installable, ProtoError};
use std::path::{Path, PathBuf};

#[cfg(not(windows))]
fn get_bin_name(bin: &str) -> String {
    bin.to_owned()
}

#[cfg(windows)]
fn get_bin_name(bin: &str) -> String {
    format!("{}.cmd", bin)
}

#[async_trait]
impl Executable<'_> for NodeDependencyManager {
    async fn find_bin_path(&mut self) -> Result<(), ProtoError> {
        let install_dir = self.get_install_dir()?;

        let bin_path = install_dir.join(match self.type_of {
            NodeDependencyManagerType::Npm => format!("bin/{}", get_bin_name("npm")),
            NodeDependencyManagerType::Pnpm => "bin/pnpm.cjs".to_owned(),
            NodeDependencyManagerType::Yarn => format!("bin/{}", get_bin_name("yarn")),
        });

        if bin_path.exists() {
            self.bin_path = Some(bin_path.clean());

            return Ok(());
        }

        return Err(ProtoError::ExecuteMissingBin(
            self.get_name(),
            install_dir.join(format!("bin/{}", self.package_name)),
        ));
    }

    fn get_bin_path(&self) -> Result<&Path, ProtoError> {
        match self.bin_path.as_ref() {
            Some(bin) => Ok(bin),
            None => Err(ProtoError::MissingTool(self.get_name())),
        }
    }

    fn get_globals_bin_dir(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .base_dir
            .parent()
            .unwrap()
            .join("node")
            .join("globals")
            .join("bin"))
    }
}
