use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use std::collections::HashMap;

static NAME: &str = "Node.js";
static BIN: &str = "node";

#[derive(Deserialize)]
pub struct PackageJson {
    pub engines: Option<HashMap<String, String>>,
    #[serde(rename = "packageManager")]
    pub package_manager: Option<String>,
    pub version: Option<String>,
}

// Metadata

#[plugin_fn]
pub fn register_tool(Json(_input): Json<ParseVersionInput>) -> FnResult<Json<ToolMetadata>> {
    Ok(Json(ToolMetadata {
        name: NAME.into(),
        type_of: PluginType::Language,
    }))
}

// Detector

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionFiles>> {
    Ok(Json(DetectVersionFiles {
        files: vec![
            ".nvmrc".into(),
            ".node-version".into(),
            "package.json".into(),
        ],
    }))
}

#[plugin_fn]
pub fn parse_version_file(Json(input): Json<ParseVersionInput>) -> FnResult<Json<ParseVersion>> {
    let mut version = None;

    if input.file == "package.json" {
        let json: PackageJson = json::from_str(&input.content)?;

        if let Some(engines) = json.engines {
            if let Some(constraint) = engines.get(BIN) {
                version = Some(constraint.to_owned());
            }
        }
    } else {
        version = Some(input.content.trim().to_owned());
    }

    Ok(Json(ParseVersion { version }))
}

// Downloader

fn map_arch(os: &str, arch: &str) -> Result<String, PluginError> {
    let arch = match arch {
        "arm" => "armv7l".into(),
        "aarch64" => "arm64".into(),
        "powerpc64" => {
            if os == "linux" {
                "ppc64le".into()
            } else {
                "ppc64".into()
            }
        }
        "s390x" => "s390x".into(),
        "x86_64" => "x64".into(),
        "x86" => "x86".into(),
        other => {
            return Err(PluginError::UnsupportedArchitecture {
                tool: NAME.into(),
                arch: other.into(),
            });
        }
    };

    Ok(arch)
}

#[plugin_fn]
pub fn register_install_params(
    Json(input): Json<EnvironmentInput>,
) -> FnResult<Json<InstallParams>> {
    let version = input.version;
    let arch = map_arch(&input.os, &input.arch)?;

    let prefix = match input.os.as_str() {
        "linux" => format!("node-v{version}-linux-{arch}"),
        "macos" => format!("node-v{version}-darwin-{arch}"),
        "windows" => format!("node-v{version}-win-{arch}"),
        other => {
            return Err(PluginError::UnsupportedPlatform {
                tool: NAME.into(),
                platform: other.into(),
            })?;
        }
    };

    let filename = if input.os == "windows" {
        format!("{prefix}.zip")
    } else {
        format!("{prefix}.tar.xz")
    };

    Ok(Json(InstallParams {
        archive_prefix: Some(prefix),
        download_url: format!("https://nodejs.org/dist/v{version}/{filename}"),
        download_file: Some(filename),
        checksum_url: Some(format!("https://nodejs.org/dist/v{version}/SHASUMS256.txt")),
        ..InstallParams::default()
    }))
}

// Shimmer

#[plugin_fn]
pub fn register_shims(Json(input): Json<EnvironmentInput>) -> FnResult<Json<ShimParams>> {
    Ok(Json(ShimParams {
        global_shims: HashMap::from_iter([(
            "npx".into(),
            if input.os == "windows" {
                "npx.cmd".into()
            } else {
                "bin/npx".into()
            },
        )]),
        ..ShimParams::default()
    }))
}
