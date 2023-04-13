use rustc_hash::FxHashMap;
use serde::Deserialize;
use std::env::consts;

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PlatformSchema {
    pub arch: FxHashMap<String, String>,
    pub os: FxHashMap<String, String>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DetectorSchema {
    pub version_files: Option<Vec<String>>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct InstallSchema {
    pub archive_prefix: Option<String>,
    pub checksum_url: Option<String>,
    pub download_ext: FxHashMap<String, String>,
    pub download_url: String,
}

#[derive(Default, Deserialize)]
pub enum ToolType {
    #[default]
    Language,
    DependencyManager,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToolSchema {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_of: ToolType,
    pub platform: PlatformSchema,

    pub detect: DetectorSchema,
    pub install: InstallSchema,
}

impl ToolSchema {
    pub fn get_arch(&self) -> &str {
        self.platform.arch.get(consts::ARCH).or_else(consts::ARCH)
    }

    pub fn get_download_ext(&self) -> &str {
        self.install.download_ext.get(consts::OS).or_else("")
    }

    pub fn get_os(&self) -> &str {
        self.platform.os.get(consts::OS).or_else(consts::OS)
    }
}
