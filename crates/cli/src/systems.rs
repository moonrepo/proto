use crate::app::{App as CLI, Commands};
use crate::helpers::fetch_latest_version;
use crate::session::ProtoSession;
use miette::IntoDiagnostic;
use proto_core::flow::install::InstallOptions;
use proto_core::{
    is_offline, now, ConfigMode, ProtoEnvironment, UnresolvedVersionSpec, PROTO_CONFIG_NAME,
};
use proto_shim::get_exe_file_name;
use semver::Version;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::fs;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, instrument};

// STARTUP

#[instrument(skip_all)]
pub fn detect_proto_env(cli: &CLI) -> AppResult<ProtoEnvironment> {
    let mut env = if let Ok(sandbox) = env::var("PROTO_SANDBOX") {
        ProtoEnvironment::new_testing(&std::path::PathBuf::from(&sandbox))?
    } else {
        ProtoEnvironment::new()?
    };

    env.config_mode = cli.config_mode.unwrap_or(match cli.command {
        Commands::Activate(_)
        | Commands::Install(_)
        | Commands::Outdated(_)
        | Commands::Status(_) => ConfigMode::Upwards,
        _ => ConfigMode::UpwardsGlobal,
    });

    Ok(env)
}

// ANALYZE

#[instrument(skip_all)]
pub fn load_proto_configs(env: &ProtoEnvironment) -> AppResult {
    debug!(
        "Loading configuration in {} mode",
        env.config_mode.to_string()
    );

    env.load_config()?;

    Ok(())
}

#[instrument(skip_all)]
pub async fn download_versioned_proto_tool(session: &ProtoSession) -> AppResult {
    let config = session
        .env
        .load_config_manager()?
        .get_merged_config_without_global()?;

    if let Some(version) = config.versions.get("proto") {
        // Only support fully-qualified versions as we need to prepend the
        // tool directory into PATH, which doesn't support requirements
        if !matches!(version, UnresolvedVersionSpec::Semantic(_)) {
            return Ok(());
        }

        let mut tool = session.load_proto_tool().await?;

        if !tool.is_installed() {
            debug!(
                version = version.to_string(),
                "Downloading a versioned proto because it was configured in {}", PROTO_CONFIG_NAME
            );

            tool.setup(version, InstallOptions::default()).await?;
        }
    }

    Ok(())
}

// EXECUTE

#[instrument(skip_all)]
pub fn clean_proto_backups(env: &ProtoEnvironment) -> AppResult {
    for bin_name in [get_exe_file_name("proto"), get_exe_file_name("proto-shim")] {
        let backup_path = env.store.bin_dir.join(format!("{bin_name}.backup"));

        if backup_path.exists() {
            let _ = fs::remove_file(backup_path);
        }
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn check_for_new_version(env: Arc<ProtoEnvironment>) -> AppResult {
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
    let current_version = env!("CARGO_PKG_VERSION");

    debug!(current_version, "Checking for a new version of proto");

    let Ok(latest_version) = fetch_latest_version().await else {
        return Ok(());
    };

    let local_version = Version::parse(current_version).into_diagnostic()?;
    let remote_version = Version::parse(&latest_version).into_diagnostic()?;

    if remote_version > local_version {
        debug!(latest_version = &latest_version, "Found a newer version");

        println!(
            "✨ There's a new version of proto available, {} (currently on {})",
            color::hash(remote_version.to_string()),
            color::muted_light(local_version.to_string()),
        );

        println!(
            "✨ Run {} or install from {}",
            color::shell("proto upgrade"),
            color::url("https://moonrepo.dev/docs/proto/install"),
        );

        println!();
    }

    // And write the cache
    fs::write_file(cache_file, now().to_string())?;

    Ok(())
}
