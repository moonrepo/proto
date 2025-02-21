use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn from_virtual_path(path: String) -> String;
    fn get_env_var(name: String) -> String;
    fn host_log(input: Json<HostLogInput>);
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
    fn set_env_var(name: String, value: String);
    fn to_virtual_path(path: String) -> String;
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
pub fn testing_macros(_: ()) -> FnResult<()> {
    // Errors
    let _ = plugin_err!(PluginError::Message("Error".into()));
    let _ = plugin_err!(code = 2, "Error");
    let _ = plugin_err!(code = 3, "Error {}", "arg");
    let _ = plugin_err!("Error");
    let _ = plugin_err!("Error {}", "arg");

    // Commands
    let args = ["a", "b", "c"];

    exec_command!("git");
    exec_command!("git", args);
    exec_command!("git", ["a", "b", "c"]);
    exec_command!(input, ExecCommandInput::default());
    exec_command!(pipe, "git");
    exec_command!(pipe, "git", args);
    exec_command!(pipe, "git", ["a", "b", "c"]);
    exec_command!(inherit, "git");
    exec_command!(inherit, "git", args);
    exec_command!(inherit, "git", ["a", "b", "c"]);
    let _ = exec_command!(raw, ExecCommandInput::default());
    let _ = exec_command!(raw, "git");
    let _ = exec_command!(raw, "git", args);
    let _ = exec_command!(raw, "git", ["a", "b", "c"]);

    // Requests
    send_request!("https://some/url");
    send_request!(input, SendRequestInput::new("https://some/url"));

    // Env vars
    let name = "VAR";

    let _ = host_env!("VAR");
    let _ = host_env!(name);
    host_env!("VAR", "value");
    host_env!("VAR", name);
    host_env!(name, name);
    host_env!(name, "value");

    // Logging
    host_log!("Message");
    host_log!("Message {} {} {}", 1, 2, 3);
    host_log!(input, HostLogInput::default());
    host_log!(stdout, "Message");
    host_log!(stdout, "Message {} {} {}", 1, 2, 3);
    host_log!(stderr, "Message");
    host_log!(stderr, "Message {} {} {}", 1, 2, 3);

    // Paths
    let path = "/proto/path";
    let pathbuf = PathBuf::from("/proto/buf");

    let _ = real_path!("/proto/dir");
    let _ = real_path!(path);
    let _ = real_path!(buf, pathbuf);
    let _ = virtual_path!("/proto/dir");
    let _ = virtual_path!(path);
    let _ = virtual_path!(buf, pathbuf);

    Ok(())
}

#[plugin_fn]
pub fn register_tool(_: ()) -> FnResult<Json<RegisterToolOutput>> {
    host_log!(stdout, "Registering tool");

    let config = get_tool_config::<WasmTestConfig>()?;

    host_log!("Config = {:?}", config);

    Ok(Json(RegisterToolOutput {
        name: "WASM Test".into(),
        type_of: PluginType::CommandLine,
        ..RegisterToolOutput::default()
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
    let env = get_host_environment()?;
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
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        globals_lookup_dirs: vec!["$WASM_ROOT/bin".into(), "$HOME/.wasm/bin".into()],
        exes: HashMap::from_iter([
            (
                "node".into(),
                ExecutableConfig::new_primary(env.os.for_native("bin/node", "node.exe")),
            ),
            ("global1".into(), ExecutableConfig::new("bin/global1")),
        ]),
        ..LocateExecutablesOutput::default()
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
    let response: Vec<NodeDistVersion> = fetch_json("https://nodejs.org/dist/index.json")?;

    for (index, item) in response.iter().enumerate() {
        let version = Version::parse(&item.version[1..])?;

        if index == 0 {
            output.latest = Some(UnresolvedVersionSpec::Semantic(SemVer(version.clone())));
        }

        output.versions.push(VersionSpec::Semantic(SemVer(version)));
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
