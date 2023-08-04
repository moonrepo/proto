use crate::helpers::{get_home_dir, get_root};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ProtoEnvironment {
    pub bin_dir: PathBuf,
    pub plugins_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub tools_dir: PathBuf,
    pub home: PathBuf,
    pub root: PathBuf,
}

impl ProtoEnvironment {
    pub fn new() -> miette::Result<Self> {
        Self::from(get_root()?)
    }

    pub fn new_testing(sandbox: &Path) -> Self {
        let mut env = Self::from(sandbox.join(".proto")).unwrap();
        env.home = sandbox.join(".home");
        env
    }

    pub fn from<P: AsRef<Path>>(root: P) -> miette::Result<Self> {
        let root = root.as_ref();

        Ok(ProtoEnvironment {
            bin_dir: root.join("bin"),
            plugins_dir: root.join("plugins"),
            temp_dir: root.join("temp"),
            tools_dir: root.join("tools"),
            home: get_home_dir()?,
            root: root.to_owned(),
        })
    }
}

impl AsRef<ProtoEnvironment> for ProtoEnvironment {
    fn as_ref(&self) -> &ProtoEnvironment {
        self
    }
}
