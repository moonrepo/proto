use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

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

// Installer

fn map_arch(os: HostOS, arch: HostArch) -> Result<String, PluginError> {
    let arch = match arch {
        HostArch::Arm => "armv7l".into(),
        HostArch::Arm64 => "arm64".into(),
        HostArch::Powerpc64 => {
            if os == HostOS::Linux {
                "ppc64le".into()
            } else {
                "ppc64".into()
            }
        }
        HostArch::S390x => "s390x".into(),
        HostArch::X64 => "x64".into(),
        HostArch::X86 => "x86".into(),
        other => {
            return Err(PluginError::UnsupportedArchitecture {
                tool: NAME.into(),
                arch: format!("{:?}", other),
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
    let arch = map_arch(input.env.os, input.env.arch)?;

    let prefix = match input.env.os {
        HostOS::Linux => format!("node-v{version}-linux-{arch}"),
        HostOS::MacOS => format!("node-v{version}-darwin-{arch}"),
        HostOS::Windows => format!("node-v{version}-win-{arch}"),
        other => {
            return Err(PluginError::UnsupportedPlatform {
                tool: NAME.into(),
                platform: format!("{:?}", other),
            })?;
        }
    };

    let filename = if input.env.os == HostOS::Windows {
        format!("{prefix}.zip")
    } else {
        format!("{prefix}.tar.xz")
    };

    Ok(Json(InstallParamsOutput {
        archive_prefix: Some(prefix),
        bin_path: Some(PathBuf::from(if input.env.os == HostOS::Windows {
            format!("{}.exe", BIN)
        } else {
            format!("bin/{}", BIN)
        })),
        download_url: format!("https://nodejs.org/dist/v{version}/{filename}"),
        download_name: Some(filename),
        checksum_url: Some(format!("https://nodejs.org/dist/v{version}/SHASUMS256.txt")),
        ..InstallParamsOutput::default()
    }))
}

// Resolver

#[derive(Deserialize)]
#[serde(untagged)]
enum NodeDistLTS {
    Name(String),
    State(bool),
}

#[derive(Deserialize)]
struct NodeDistVersion {
    lts: NodeDistLTS,
    version: String, // Starts with v
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let response: Vec<NodeDistVersion> = fetch_url("https://nodejs.org/dist/index.json")?;

    for (index, item) in response.iter().enumerate() {
        let version = Version::parse(&item.version[1..])?;

        // First item is always the latest
        if index == 0 {
            output.latest = Some(version.clone());
        }

        if let NodeDistLTS::Name(alias) = &item.lts {
            let alias = alias.to_lowercase();

            // The first encounter of an lts in general is the latest stable
            if !output.aliases.contains_key("stable") {
                output.aliases.insert("stable".into(), version.clone());
            }

            // The first encounter of an lts is the latest version for that alias
            if !output.aliases.contains_key(&alias) {
                output.aliases.insert(alias.clone(), version.clone());
            }
        }

        output.versions.push(version);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let mut output = ResolveVersionOutput::default();

    if input.initial == "node" {
        output.candidate = Some("latest".into());

    // Stable version is the first with an LTS
    } else if input.initial == "lts-*" || input.initial == "lts/*" {
        output.candidate = Some("stable".into());

        // Find the first version with a matching LTS
    } else if input.initial.starts_with("lts-") || input.initial.starts_with("lts/") {
        output.candidate = Some(input.initial[4..].to_owned());
    }

    Ok(Json(output))
}

// Shimmer

#[plugin_fn]
pub fn register_shims(Json(input): Json<ShimParamsInput>) -> FnResult<Json<ShimParamsOutput>> {
    Ok(Json(ShimParamsOutput {
        global_shims: HashMap::from_iter([(
            "npx".into(),
            if input.env.os == HostOS::Windows {
                "npx.cmd".into()
            } else {
                "bin/npx".into()
            },
        )]),
        ..ShimParamsOutput::default()
    }))
}
