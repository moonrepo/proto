use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use starbase_utils::fs;

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    initialize_tracing();

    Ok(Json(RegisterToolOutput {
        name: input.id.clone(),
        type_of: PluginType::CommandLine,
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        requires: if input.id == "moonbase" {
            vec!["moonstone".into()]
        } else {
            vec![]
        },
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(
    Json(_): Json<DetectVersionInput>,
) -> FnResult<Json<DetectVersionOutput>> {
    let id = get_plugin_id()?;

    Ok(Json(DetectVersionOutput {
        files: vec![format!(".{id}rc"), format!(".{id}.json")],
        ignore: vec![],
    }))
}

#[derive(Deserialize)]
struct VersionJson {
    version: Option<String>,
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;
    let id = get_plugin_id()?;

    if input.file == format!(".{id}.json") {
        let json: VersionJson = json::from_str(&input.content)?;

        if let Some(constraint) = json.version {
            version = Some(UnresolvedVersionSpec::parse(constraint)?);
        }
    } else {
        version = Some(UnresolvedVersionSpec::parse(input.content.trim())?);
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut tags = vec![];

    for major in 1..=5 {
        for minor in 0..=10 {
            for patch in 0..=15 {
                tags.push(format!("{major}.{minor}.{patch}"));
            }
        }
    }

    tags.push("6.0.0-alpha.0".into());
    tags.push("6.0.0-beta.1".into());
    tags.push("6.0.0-beta.2".into());
    tags.push("6.0.0-rc.0".into());
    tags.push("6.0.0-rc.1".into());
    tags.push("canary".into());

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let mut output = ResolveVersionOutput::default();

    if let UnresolvedVersionSpec::Alias(alias) = input.initial {
        let candidate = if alias == "stable" {
            "5.0.0"
        } else if alias == "unstable" {
            "6.0.0-rc.1"
        } else if alias == "legacy" {
            "4.10.15"
        } else {
            return Ok(Json(output));
        };

        output.candidate = Some(UnresolvedVersionSpec::parse(candidate)?);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn native_install(
    Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
    let id = get_plugin_id()?;
    let env = get_host_environment()?;
    let version = &input.context.version;

    check_supported_os_and_arch(
        &id,
        &env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64],
        ],
    )?;

    // Check the version is valid and error otherwise
    // This is necessary since this would typically be handled by prebuilts
    if let Some(inner) = version.as_version()
        && (inner.major < 1 || inner.major > 6 || inner.minor > 10 || inner.patch > 15)
    {
        return Err(plugin_err!("Invalid version {version}"));
    }

    // Create the primary executable
    fs::write_file(input.install_dir.join(env.os.get_exe_name(&id)), "")?;

    // Create other executables
    let lib_dir = input.install_dir.join("lib");

    fs::create_dir_all(&lib_dir)?;
    fs::write_file(lib_dir.join(env.os.get_exe_name(format!("{id}-dbg"))), "")?;
    fs::write_file(lib_dir.join(env.os.get_exe_name(format!("{id}-fmt"))), "")?;
    fs::write_file(lib_dir.join(env.os.get_exe_name(format!("{id}x"))), "")?;

    // We need a checksum for tests to work,
    // so base it on the version so every hash is different
    let sha = Sha256::digest(input.context.version.to_string());
    let hash = format!("{sha:x}");

    Ok(Json(NativeInstallOutput {
        checksum: Some(Checksum::sha256(hash)),
        installed: true,
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let id = get_plugin_id()?;
    let env = get_host_environment()?;
    let mut output = LocateExecutablesOutput::default();

    // Executables
    output.exes.insert(
        id.clone(),
        ExecutableConfig::new_primary(env.os.get_exe_name(&id)),
    );

    output.exes.insert(
        format!("{id}x"),
        ExecutableConfig::new(env.os.get_exe_name(format!("lib/{id}x"))),
    );

    output.exes_dirs.push("lib".into());

    // Globals
    output
        .globals_lookup_dirs
        .push(format!("${id}_BIN").to_uppercase().replace('-', "_"));

    output.globals_lookup_dirs.push(format!("$HOME/.{id}/bin"));

    Ok(Json(output))
}
