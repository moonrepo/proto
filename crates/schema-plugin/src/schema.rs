use rustc_hash::FxHashMap;
use serde::Deserialize;
use std::env::consts;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PlatformSchema {
    pub arch: FxHashMap<String, String>,
    pub os: FxHashMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DetectorSchema {
    pub version_files: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ExecuteSchema {
    pub bin_path: Option<FxHashMap<String, String>>,
    pub globals_dir: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct InstallSchema {
    pub archive_prefix: Option<String>,
    pub checksum_url: Option<String>,
    pub download_ext: FxHashMap<String, String>,
    pub download_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ResolveSchema {
    // Manifest
    pub manifest_url: Option<String>,
    pub manifest_version_key: String,
    // Tags
    pub git_url: Option<String>,
    pub git_tag_pattern: String,
}

impl Default for ResolveSchema {
    fn default() -> Self {
        ResolveSchema {
            manifest_url: None,
            manifest_version_key: "version".to_string(),
            git_url: None,
            git_tag_pattern: r"(\d+)\.(\d+)\.(\d+)".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ShimSchema {
    pub local: bool,
    pub global: bool,
    pub parent_bin: Option<String>,
}

impl Default for ShimSchema {
    fn default() -> Self {
        ShimSchema {
            local: false,
            global: true,
            parent_bin: None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub enum ToolType {
    #[default]
    Language,
    DependencyManager,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToolSchema {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_of: ToolType,
    pub platform: PlatformSchema,

    pub detect: DetectorSchema,
    pub execute: ExecuteSchema,
    pub install: InstallSchema,
    pub resolve: ResolveSchema,
    pub shim: ShimSchema,
}

impl ToolSchema {
    pub fn get_arch(&self) -> &str {
        self.platform
            .arch
            .get(consts::ARCH)
            .map(|v| v.as_ref())
            .unwrap_or(consts::ARCH)
    }

    pub fn get_download_ext(&self) -> &str {
        self.install
            .download_ext
            .get(consts::OS)
            .map(|v| v.as_ref())
            .unwrap_or("")
    }

    pub fn get_os(&self) -> &str {
        self.platform
            .os
            .get(consts::OS)
            .map(|v| v.as_ref())
            .unwrap_or(consts::OS)
    }
}
