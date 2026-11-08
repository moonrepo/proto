use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use starbase_utils::fs;

// Mimics backends (like `asdf:golang`) whose executables are not
// named after the tool, so none of them are marked as primary
const NO_PRIMARY_ID: &str = "protolith";

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    initialize_tracing();

    Ok(Json(RegisterToolOutput {
        name: input.id.to_string(),
        type_of: PluginType::CommandLine,
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        requires: if input.id == "moonbase" {
            vec!["moonstone".into()]
        } else {
            vec![]
        },
        lock_options: ToolLockOptions {
            ignore_os_arch: input.id == "protoform",
            ..Default::default()
        },
        self_upgrade_commands: vec!["self upgrade".into()],
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(
    Json(_): Json<DetectVersionInput>,
) -> FnResult<Json<DetectVersionOutput>> {
    let id = get_plugin_id()?;

    Ok(Json(DetectVersionOutput {
        files: vec![
            format!(".{id}rc"),
            format!(".{id}.json"),
            format!(".{id}.stop"),
            format!(".{id}.after-stop"),
        ],
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
    let mut stop = false;
    let id = get_plugin_id()?;
    let content = fs::read_file(&input.path)?;

    if input.file == format!(".{id}.json") {
        let json: VersionJson = json::from_str(&content)?;

        if let Some(constraint) = json.version {
            version = Some(UnresolvedVersionSpec::parse(constraint)?);
        }
    } else if input.file == format!(".{id}.stop") {
        stop = true;

        // An empty file has no version, but still shadows the files after it
        if !content.trim().is_empty() {
            version = Some(UnresolvedVersionSpec::parse(content.trim())?);
        }
    } else {
        version = Some(UnresolvedVersionSpec::parse(content.trim())?);
    }

    Ok(Json(ParseVersionFileOutput { version, stop }))
}

#[plugin_fn]
pub fn pin_version(Json(input): Json<PinVersionInput>) -> FnResult<Json<PinVersionOutput>> {
    let mut output = PinVersionOutput::default();
    let id = get_plugin_id()?;

    match input.version {
        UnresolvedVersionSpec::Alias(_) => {
            output.error = Some("Aliases are not supported.".into());
        }
        _ => {
            let file = input.dir.join(format!(".{id}-version"));

            match fs::write_file(&file, input.version.to_string()) {
                Ok(_) => {
                    output.file = Some(file);
                    output.pinned = true;
                }
                Err(error) => {
                    output.error = Some(error.to_string());
                    output.pinned = false;
                }
            }
        }
    };

    Ok(Json(output))
}

#[plugin_fn]
pub fn unpin_version(Json(input): Json<UnpinVersionInput>) -> FnResult<Json<UnpinVersionOutput>> {
    let mut output = UnpinVersionOutput::default();
    let id = get_plugin_id()?;
    let file = input.dir.join(format!(".{id}-version"));

    if file.exists() {
        let version = fs::read_file(&file)?;

        fs::remove_file(&file)?;

        output.file = Some(file);
        output.unpinned = true;
        output.version = Some(UnresolvedVersionSpec::parse(&version)?);
    } else {
        output.error = Some("Version file does not exist.".into());
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn load_versions(Json(input): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    // Simulate an unreachable remote, so that tests can verify the
    // behavior when available versions fail to load
    if input.initial.get_scope() == Some("unreachable") {
        return Err(plugin_err!("Failed to make HTTP request."));
    }

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

    match &input.initial {
        UnresolvedVersionSpec::Alias(alias) => {
            // Return a version as-is, without it being validated against
            // the list of available versions
            if alias == "explicit" {
                output.version = Some(VersionSpec::parse("7.7.7")?);

                return Ok(Json(output));
            }

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
        // Remap a fully qualified version that doesn't exist to one that does,
        // so that we can verify this function is always called
        UnresolvedVersionSpec::Version(version) if version.to_string() == "9.9.9" => {
            output.candidate = Some(UnresolvedVersionSpec::parse("5.0.0")?);
        }
        _ => {}
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
        env,
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

    // Std doesn't create parent directories, so this verifies
    // that the host created the temp directory
    std::fs::write(input.temp_dir.join("native_install"), "")?;

    // Create the primary executable
    if id != NO_PRIMARY_ID {
        fs::write_file(input.install_dir.join(env.os.get_exe_name(&id)), "")?;
    }

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
    if id == NO_PRIMARY_ID {
        output.exes.insert(
            format!("{id}-fmt"),
            ExecutableConfig::new(env.os.get_exe_name(format!("lib/{id}-fmt"))),
        );
    } else {
        output.exes.insert(
            id.to_string(),
            ExecutableConfig::new_primary(env.os.get_exe_name(&id)),
        );
    }

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
