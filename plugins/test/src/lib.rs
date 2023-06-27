use extism_pdk::*;
use proto_pdk::*;
use std::collections::HashMap;

#[plugin_fn]
pub fn register_tool(_: ()) -> FnResult<Json<ToolMetadata>> {
    Ok(Json(ToolMetadata {
        name: "WASM Test".into(),
        type_of: PluginType::CLI,
    }))
}

// Detector

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionFiles>> {
    Ok(Json(DetectVersionFiles {
        files: vec![".proto-wasm-version".into(), ".protowasmrc".into()],
    }))
}

#[plugin_fn]
pub fn parse_version_file(Json(input): Json<ParseVersionInput>) -> FnResult<Json<ParseVersion>> {
    let mut version = None;

    if input.file == ".proto-wasm-version" {
        if input.content.starts_with("version=") {
            version = Some(input.content[8..].into());
        }
    } else {
        version = Some(input.content);
    }

    Ok(Json(ParseVersion { version }))
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
pub fn register_install_params(
    Json(input): Json<EnvironmentInput>,
) -> FnResult<Json<InstallParams>> {
    let version = input.version;
    let arch = map_arch(&input.arch);

    let prefix = match input.os.as_str() {
        "linux" => format!("node-v{version}-linux-{arch}"),
        "macos" => format!("node-v{version}-darwin-{arch}"),
        "windows" => format!("node-v{version}-win-{arch}"),
        _ => unimplemented!(),
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
        checksum_file: None,
    }))
}

// #[plugin_fn]
// pub fn unpack_archive(Json(input): Json<UnpackArchiveInput>) -> FnResult<()> {
//     untar(input.download_path, input.install_dir)?;
//     Ok(())
// }

// Shimmer

#[plugin_fn]
pub fn register_shims(_: ()) -> FnResult<Json<ShimParams>> {
    Ok(Json(ShimParams {
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
        ..ShimParams::default()
    }))
}

// Verifier

#[plugin_fn]
pub fn verify_checksum(Json(input): Json<VerifyChecksumInput>) -> FnResult<Json<VerifyChecksum>> {
    info!(
        "Verifying checksum of {:?} using {:?}",
        input.download_path, input.checksum_path
    );

    Ok(Json(VerifyChecksum {
        verified: input.download_path.exists()
            && input.checksum_path.exists()
            && input.version == "20.0.0",
    }))
}
