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
pub fn register_tool(Json(_input): Json<ToolMetadataInput>) -> FnResult<Json<ToolMetadataOutput>> {
    Ok(Json(ToolMetadataOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        ..ToolMetadataOutput::default()
    }))
}

// Detector

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![
            ".nvmrc".into(),
            ".node-version".into(),
            "package.json".into(),
        ],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionInput>,
) -> FnResult<Json<ParseVersionOutput>> {
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

    Ok(Json(ParseVersionOutput { version }))
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
pub fn register_install(
    Json(input): Json<InstallParamsInput>,
) -> FnResult<Json<InstallParamsOutput>> {
    let version = input.env.version;
    let arch = map_arch(&input.env.os, &input.env.arch)?;

    let prefix = match input.env.os.as_str() {
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

    let filename = if input.env.os == "windows" {
        format!("{prefix}.zip")
    } else {
        format!("{prefix}.tar.xz")
    };

    Ok(Json(InstallParamsOutput {
        archive_prefix: Some(prefix),
        bin_path: Some(if input.env.os == "windows" {
            format!("{}.exe", BIN)
        } else {
            format!("bin/{}", BIN)
        }),
        download_url: format!("https://nodejs.org/dist/v{version}/{filename}"),
        download_name: Some(filename),
        checksum_url: Some(format!("https://nodejs.org/dist/v{version}/SHASUMS256.txt")),
        ..InstallParamsOutput::default()
    }))
}

// Shimmer

#[plugin_fn]
pub fn register_shims(Json(input): Json<ShimParamsInput>) -> FnResult<Json<ShimParamsOutput>> {
    Ok(Json(ShimParamsOutput {
        global_shims: HashMap::from_iter([(
            "npx".into(),
            if input.env.os == "windows" {
                "npx.cmd".into()
            } else {
                "bin/npx".into()
            },
        )]),
        ..ShimParamsOutput::default()
    }))
}
