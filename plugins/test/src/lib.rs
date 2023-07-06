use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use std::collections::HashMap;

#[plugin_fn]
pub fn register_tool(_: ()) -> FnResult<Json<ToolMetadataOutput>> {
    Ok(Json(ToolMetadataOutput {
        name: "WASM Test".into(),
        type_of: PluginType::CLI,
        ..ToolMetadataOutput::default()
    }))
}

// Detector

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".proto-wasm-version".into(), ".protowasmrc".into()],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionInput>,
) -> FnResult<Json<ParseVersionOutput>> {
    let mut version = None;

    if input.file == ".proto-wasm-version" {
        if input.content.starts_with("version=") {
            version = Some(input.content[8..].into());
        }
    } else {
        version = Some(input.content);
    }

    Ok(Json(ParseVersionOutput { version }))
}

// Downloader

fn map_arch(arch: &str) -> String {
    match arch {
        "aarch64" => "arm64".into(),
        "x86_64" => "x64".into(),
        "x86" => "x86".into(),
        other => other.into(),
    }
}

#[plugin_fn]
pub fn register_install(
    Json(input): Json<InstallParamsInput>,
) -> FnResult<Json<InstallParamsOutput>> {
    let version = input.env.version;
    let arch = map_arch(&input.env.arch);

    let prefix = match input.env.os.as_str() {
        "linux" => format!("node-v{version}-linux-{arch}"),
        "macos" => format!("node-v{version}-darwin-{arch}"),
        "windows" => format!("node-v{version}-win-{arch}"),
        _ => unimplemented!(),
    };

    let filename = if input.env.os == "windows" {
        format!("{prefix}.zip")
    } else {
        format!("{prefix}.tar.xz")
    };

    Ok(Json(InstallParamsOutput {
        archive_prefix: Some(prefix),
        bin_path: Some(if input.env.os == "windows" {
            "node.exe".into()
        } else {
            "bin/node".into()
        }),
        download_url: format!("https://nodejs.org/dist/v{version}/{filename}"),
        download_name: Some(filename),
        checksum_url: Some(format!("https://nodejs.org/dist/v{version}/SHASUMS256.txt")),
        checksum_name: None,
    }))
}

// #[plugin_fn]
// pub fn unpack_archive(Json(input): Json<UnpackArchiveInput>) -> FnResult<()> {
//     untar(input.download_path, input.install_dir)?;
//     Ok(())
// }

#[plugin_fn]
pub fn find_bins(Json(_): Json<ExecuteParamsInput>) -> FnResult<Json<ExecuteParamsOutput>> {
    Ok(Json(ExecuteParamsOutput {
        globals_lookup_dirs: vec!["$WASM_ROOT/bin".into(), "$HOME/.wasm/bin".into()],
        ..ExecuteParamsOutput::default()
    }))
}

// Resolver

#[derive(Deserialize)]
struct NodeDistVersion {
    version: String, // Starts with v
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let response: Vec<NodeDistVersion> = fetch_url("https://nodejs.org/dist/index.json")?;

    for (index, item) in response.iter().enumerate() {
        let version = Version::parse(&item.version[1..])?;

        if index == 0 {
            output.latest = Some(version.clone());
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
    }

    Ok(Json(output))
}

// Shimmer

#[plugin_fn]
pub fn register_shims(_: ()) -> FnResult<Json<ShimParamsOutput>> {
    Ok(Json(ShimParamsOutput {
        global_shims: HashMap::from_iter([("global1".into(), "bin/global1".into())]),
        local_shims: HashMap::from_iter([
            (
                "local1".into(),
                ShimConfig {
                    bin_path: "bin/local1".into(),
                    parent_bin: Some("node".into()),
                    ..Default::default()
                },
            ),
            (
                "local2".into(),
                ShimConfig {
                    bin_path: "local2.js".into(),
                    parent_bin: None,
                    ..Default::default()
                },
            ),
        ]),
        ..ShimParamsOutput::default()
    }))
}

// Verifier

#[plugin_fn]
pub fn verify_checksum(
    Json(input): Json<VerifyChecksumInput>,
) -> FnResult<Json<VerifyChecksumOutput>> {
    info!(
        "Verifying checksum of {:?} using {:?}",
        input.download_file, input.checksum_file
    );

    Ok(Json(VerifyChecksumOutput {
        verified: input.download_file.exists()
            && input.checksum_file.exists()
            && input.env.version == "20.0.0",
    }))
}
