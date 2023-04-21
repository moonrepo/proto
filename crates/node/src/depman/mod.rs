mod detect;
mod download;
mod execute;
mod install;
mod resolve;
mod shim;
mod verify;

use proto_core::{Describable, Proto, Tool};
// use resolve::NDMVersionDist;
use std::{
    any::Any,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum NodeDependencyManagerType {
    Npm,
    Pnpm,
    Yarn,
}

impl NodeDependencyManagerType {
    pub fn get_package_name(&self) -> String {
        match self {
            NodeDependencyManagerType::Npm => "npm".into(),
            NodeDependencyManagerType::Pnpm => "pnpm".into(),
            NodeDependencyManagerType::Yarn => "yarn".into(),
        }
    }
}

#[derive(Debug)]
pub struct NodeDependencyManager {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    // pub dist: Option<NDMVersionDist>,
    pub package_name: String,
    pub shim_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub type_of: NodeDependencyManagerType,
    pub version: Option<String>,
}

impl NodeDependencyManager {
    pub fn new<P: AsRef<Proto>>(proto: P, type_of: NodeDependencyManagerType) -> Self {
        let proto = proto.as_ref();
        let package_name = type_of.get_package_name();

        NodeDependencyManager {
            base_dir: proto.tools_dir.join(&package_name),
            bin_path: None,
            // dist: None,
            shim_path: None,
            temp_dir: proto.temp_dir.join(&package_name),
            type_of,
            version: None,
            package_name,
        }
    }

    // pub fn get_dist(&self) -> &NDMVersionDist {
    //     self.dist
    //         .as_ref()
    //         .expect("Distribution info not defined for node dependency manager!")
    // }
}

impl Describable<'_> for NodeDependencyManager {
    fn get_bin_name(&self) -> &str {
        &self.package_name
    }

    fn get_name(&self) -> String {
        self.type_of.get_package_name()
    }
}

impl Tool<'_> for NodeDependencyManager {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_tool_dir(&self) -> &Path {
        &self.base_dir
    }
}
