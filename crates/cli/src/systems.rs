use crate::app::{App as CLI, Commands};
use crate::helpers::fetch_latest_version;
use proto_core::{ConfigMode, ProtoConsole, ProtoEnvironment, is_offline, now};
use proto_shim::get_exe_file_name;
use semver::Version;
use starbase_styles::color;
use starbase_utils::fs;
use std::env;
use std::time::Duration;
use tracing::{debug, instrument};

// STARTUP

#[instrument(skip_all)]
pub fn detect_proto_env(cli: &CLI) -> miette::Result<ProtoEnvironment> {
    #[cfg(debug_assertions)]
    let mut env = if let Ok(sandbox) = env::var("PROTO_SANDBOX") {
        ProtoEnvironment::new_testing(&std::path::PathBuf::from(&sandbox))?
    } else {
        ProtoEnvironment::new()?
    };

    #[cfg(not(debug_assertions))]
    let mut env = ProtoEnvironment::new()?;

    env.config_mode = cli.config_mode.unwrap_or(match cli.command {
        Commands::Activate(_)
        | Commands::Install(_)
        | Commands::Outdated(_)
        | Commands::Status(_) => ConfigMode::Upwards,
        _ => ConfigMode::UpwardsGlobal,
    });

    Ok(env)
}

// Temporary!
#[instrument(skip_all)]
pub fn remove_proto_shims(env: &ProtoEnvironment) -> miette::Result<()> {
    for shim_name in [get_exe_file_name("proto"), get_exe_file_name("proto-shim")] {
        let shim_path = env.store.shims_dir.join(shim_name);

        if shim_path.exists() {
            let _ = fs::remove_file(shim_path);
        }
    }

    Ok(())
}

// ANALYZE

#[instrument(skip_all)]
pub fn load_proto_configs(env: &ProtoEnvironment) -> miette::Result<()> {
    debug!(
        working_dir = ?env.working_dir,
        "Loading configuration in {} mode",
        env.config_mode.to_string()
    );

    env.load_config()?;

    Ok(())
}

// EXECUTE

#[instrument(skip_all)]
pub fn clean_proto_backups(env: &ProtoEnvironment) -> miette::Result<()> {
    for bin_name in [get_exe_file_name("proto"), get_exe_file_name("proto-shim")] {
        let backup_path = env.store.bin_dir.join(format!("{bin_name}.backup"));

        if backup_path.exists() {
            let _ = fs::remove_file(backup_path);
        }
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn check_for_new_version(
    env: &ProtoEnvironment,
    console: &ProtoConsole,
    local_version: &Version,
) -> miette::Result<()> {
    if
    // Don't check when running tests
    env.test_only ||
        // Or when explicitly disabled
        env::var("PROTO_VERSION_CHECK").is_ok_and(|var| var == "0" || var == "false") ||
            // Or when printing formatted output
            env::args().any(|arg| arg == "--json") ||
                // Or when offline
                is_offline()
    {
        return Ok(());
    }

    // Only check every 12 hours instead of every invocation
    let cache_file = env.store.temp_dir.join(".last-version-check");

    if cache_file.exists() {
        if let Some(last_check) = fs::read_file(&cache_file)
            .ok()
            .and_then(|cache| cache.parse::<u128>().ok())
        {
            if (last_check + Duration::from_secs(43200).as_millis()) > now() {
                return Ok(());
            }
        }
    }

    // Otherwise fetch and compare versions
    debug!(
        current_version = local_version.to_string(),
        "Checking for a new version of proto"
    );

    let Ok(remote_version) = fetch_latest_version().await else {
        return Ok(());
    };

    if local_version < &remote_version {
        debug!(
            latest_version = remote_version.to_string(),
            "Found a newer version"
        );

        if !console.out.is_quiet() {
            console.out.write_line(format!(
                "✨ There's a new version of proto available, {} (currently on {})",
                color::hash(remote_version.to_string()),
                color::muted_light(local_version.to_string()),
            ))?;

            console.out.write_line(format!(
                "✨ Run {} or install from {}",
                color::shell("proto upgrade"),
                color::url("https://moonrepo.dev/docs/proto/install"),
            ))?;

            console.out.write_newline()?;
        }
    }

    // And write the cache
    fs::write_file(cache_file, now().to_string())?;

    Ok(())
}
