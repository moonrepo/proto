use rustc_hash::FxHashMap;
use serde::Deserialize;
use std::env::consts;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PlatformMapper {
    pub archive_prefix: Option<String>,
    pub bin_path: Option<String>,
    pub checksum_file: Option<String>,
    pub download_file: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DetectSchema {
    pub version_files: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ExecuteSchema {
    pub globals_dir: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct InstallSchema {
    pub arch: FxHashMap<String, String>,
    pub checksum_url: Option<String>,
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
            git_tag_pattern: r"^v?(\d+)\.(\d+)\.(\d+)".to_string(),
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
pub struct Schema {
    pub bin: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_of: ToolType,
    pub platform: FxHashMap<String, PlatformMapper>,

    pub detect: DetectSchema,
    pub execute: ExecuteSchema,
    pub install: InstallSchema,
    pub resolve: ResolveSchema,
    pub shim: ShimSchema,
}

impl Schema {
    pub fn get_arch(&self) -> &str {
        self.install
            .arch
            .get(consts::ARCH)
            .map(|v| v.as_ref())
            .unwrap_or(consts::ARCH)
    }

    pub fn get_libc(&self) -> &str {
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            return if proto_core::is_musl() { "musl" } else { "gnu" };
        }

        ""
    }
}
