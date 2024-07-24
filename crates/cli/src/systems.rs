use crate::helpers::fetch_latest_version;
use miette::IntoDiagnostic;
use proto_core::{is_offline, now, ProtoEnvironment, UnresolvedVersionSpec, PROTO_CONFIG_NAME};
use proto_installer::{determine_triple, download_release, unpack_release};
use proto_shim::get_exe_file_name;
use semver::Version;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::fs;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, instrument, trace};

// STARTUP

#[instrument(skip_all)]
pub fn detect_proto_env() -> AppResult<ProtoEnvironment> {
    ProtoEnvironment::new()
}

#[instrument(skip_all)]
pub fn sync_current_proto_tool(env: &ProtoEnvironment, version: &str) -> AppResult {
    let tool_dir = env.store.inventory_dir.join("proto").join(version);

    if tool_dir.exists() {
        return Ok(());
    }

    let Ok(current_exe) = env::current_exe() else {
        return Ok(());
    };

    let exe_dir = current_exe.parent().unwrap_or(&env.store.bin_dir);

    for exe_name in [get_exe_file_name("proto"), get_exe_file_name("proto-shim")] {
        let src_file = exe_dir.join(&exe_name);
        let dst_file = tool_dir.join(&exe_name);

        if src_file.exists() && !dst_file.exists() {
            fs::copy_file(src_file, dst_file)?;
        }
    }

    Ok(())
}

// ANALYZE

#[instrument(skip_all)]
pub fn load_proto_configs(env: &ProtoEnvironment) -> AppResult {
    env.load_config()?;

    Ok(())
}

#[instrument(skip_all)]
pub async fn download_versioned_proto_tool(env: &ProtoEnvironment) -> AppResult {
    if is_offline() {
        return Ok(());
    }

    let config = env
        .load_config_manager()?
        .get_merged_config_without_global()?;

    if let Some(UnresolvedVersionSpec::Semantic(version)) = config.versions.get("proto") {
        let version = version.to_string();
        let tool_dir = env.store.inventory_dir.join("proto").join(&version);

        if tool_dir.exists() {
            return Ok(());
        }

        let triple_target = determine_triple()?;

        debug!(
            version = &version,
            install_dir = ?tool_dir,
            "Downloading a versioned proto because it was configured in {}",
            PROTO_CONFIG_NAME
        );

        unpack_release(
            download_release(
                &triple_target,
                &version,
                &env.store.temp_dir,
                |downloaded_size, total_size| {
                    trace!("Downloaded {} of {} bytes", downloaded_size, total_size);
                },
            )
            .await?,
            &tool_dir,
            &tool_dir,
            false,
        )?;
    }

    Ok(())
}

// EXECUTE

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
