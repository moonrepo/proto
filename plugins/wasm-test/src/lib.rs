use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use std::collections::HashMap;
// use std::path::PathBuf;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    // fn from_virtual_path(path: &str) -> String;
    fn get_env_var(name: &str) -> String;
    fn host_log(input: Json<HostLogInput>);
    fn set_env_var(name: &str, value: &str);
    // fn to_virtual_path(path: &str) -> String;
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct WasmTestConfig {
    number: usize,
    string: String,
    boolean: bool,
    unknown: Option<usize>,
    list: Vec<String>,
    map: HashMap<String, usize>,
}

#[plugin_fn]
pub fn register_tool(_: ()) -> FnResult<Json<ToolMetadataOutput>> {
    host_log!(stdout, "Registering tool");

    let value = host_env!("WASM_KEY");

    host_log!(stderr, "WASM_KEY = {:?}", value);
    host_env!("WASM_SOURCE", "guest");

    // let real = real_path!(PathBuf::from("/proto"));
    // let _virtual = virtual_path!(&real);

    let config = get_tool_config::<WasmTestConfig>()?;

    host_log!("Config = {:?}", config);

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
        ignore: vec!["node_modules".into()],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    if input.file == ".proto-wasm-version" {
        if input.content.starts_with("version=") {
            version = Some(UnresolvedVersionSpec::parse(&input.content[8..])?);
        }
    } else {
        version = Some(UnresolvedVersionSpec::parse(input.content)?);
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

// Downloader

fn map_arch(arch: HostArch) -> String {
    match arch {
        HostArch::Arm64 => "arm64".into(),
        HostArch::X64 => "x64".into(),
        HostArch::X86 => "x86".into(),
        _ => unimplemented!(),
    }
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_proto_environment()?;
    let version = input.context.version;
    let arch = map_arch(env.arch);

    let prefix = match env.os {
        HostOS::Linux => format!("node-v{version}-linux-{arch}"),
        HostOS::MacOS => format!("node-v{version}-darwin-{arch}"),
        HostOS::Windows => format!("node-v{version}-win-{arch}"),
        _ => unimplemented!(),
    };

    let filename = if env.os == HostOS::Windows {
        format!("{prefix}.zip")
    } else {
        format!("{prefix}.tar.xz")
    };

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(prefix),
        download_url: format!("https://nodejs.org/dist/v{version}/{filename}"),
        download_name: Some(filename),
        checksum_url: Some(format!("https://nodejs.org/dist/v{version}/SHASUMS256.txt")),
        checksum_name: None,
        checksum_public_key: None,
    }))
}

// #[plugin_fn]
// pub fn unpack_archive(Json(input): Json<UnpackArchiveInput>) -> FnResult<()> {
//     untar(input.download_path, input.install_dir)?;
//     Ok(())
// }

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_proto_environment()?;

    Ok(Json(LocateExecutablesOutput {
        globals_lookup_dirs: vec!["$WASM_ROOT/bin".into(), "$HOME/.wasm/bin".into()],
        primary: Some(ExecutableConfig::new(
            env.os.for_native("bin/node", "node.exe"),
        )),
        secondary: HashMap::from_iter([("global1".into(), ExecutableConfig::new("bin/global1"))]),
        ..LocateExecutablesOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_bins(Json(_): Json<LocateBinsInput>) -> FnResult<Json<LocateBinsOutput>> {
    let env = get_proto_environment()?;

    Ok(Json(LocateBinsOutput {
        bin_path: Some(if env.os == HostOS::Windows {
            "node.exe".into()
        } else {
            "bin/node".into()
        }),
        globals_lookup_dirs: vec!["$WASM_ROOT/bin".into(), "$HOME/.wasm/bin".into()],
        ..LocateBinsOutput::default()
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
    let response: Vec<NodeDistVersion> =
        fetch_url_with_cache("https://nodejs.org/dist/index.json")?;

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

    if let UnresolvedVersionSpec::Alias(alias) = &input.initial {
        if alias == "node" {
            output.candidate = Some(UnresolvedVersionSpec::Alias("latest".into()));
        }
    }

    Ok(Json(output))
}

// Shimmer

#[plugin_fn]
pub fn create_shims(_: ()) -> FnResult<Json<CreateShimsOutput>> {
    Ok(Json(CreateShimsOutput {
        global_shims: HashMap::from_iter([(
            "global1".into(),
            ShimConfig {
                bin_path: Some("bin/global1".into()),
                ..Default::default()
            },
        )]),
        local_shims: HashMap::from_iter([
            (
                "local1".into(),
                ShimConfig {
                    bin_path: Some("bin/local1".into()),
                    parent_bin: Some("node".into()),
                    ..Default::default()
                },
            ),
            (
                "local2".into(),
                ShimConfig {
                    bin_path: Some("local2.js".into()),
                    parent_bin: None,
                    ..Default::default()
                },
            ),
        ]),
        ..CreateShimsOutput::default()
    }))
}

// Verifier

#[plugin_fn]
pub fn verify_checksum(
    Json(input): Json<VerifyChecksumInput>,
) -> FnResult<Json<VerifyChecksumOutput>> {
    info!(
        "Verifying checksum of {:?} ({}) using {:?} ({}) ({})",
        input.download_file,
        input.download_file.exists(),
        input.checksum_file,
        input.checksum_file.exists(),
        input.context.version
    );

    Ok(Json(VerifyChecksumOutput {
        verified: input.download_file.exists()
            && input.checksum_file.exists()
            && input.context.version != "19.0.0",
    }))
}
