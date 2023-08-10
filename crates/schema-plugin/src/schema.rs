use serde::Deserialize;
use std::collections::HashMap;

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

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct InstallSchema {
    pub arch: HashMap<String, String>,
    pub checksum_url: Option<String>,
    pub download_url: String,
}

impl Default for InstallSchema {
    fn default() -> Self {
        InstallSchema {
            arch: HashMap::default(),
            checksum_url: None,
            download_url: String::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct GlobalsSchema {
    pub install_args: Option<Vec<String>>,
    pub lookup_dirs: Vec<String>,
    pub package_prefix: Option<String>,
}

impl Default for GlobalsSchema {
    fn default() -> Self {
        GlobalsSchema {
            install_args: None,
            lookup_dirs: vec![],
            package_prefix: None,
        }
    }
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
            git_tag_pattern: r"^v?((\d+)\.(\d+)\.(\d+))".to_string(),
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
#[serde(rename_all = "kebab-case")]
pub enum SchemaType {
    #[default]
    Language,
    DependencyManager,
    Cli,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Schema {
    pub name: String,
    #[serde(rename = "type")]
    pub type_of: SchemaType,
    pub platform: HashMap<String, PlatformMapper>,

    pub detect: DetectSchema,
    pub install: InstallSchema,
    pub globals: GlobalsSchema,
    pub resolve: ResolveSchema,
    pub shim: ShimSchema,
}
