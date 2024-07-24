use crate::helpers::fetch_latest_version;
use miette::IntoDiagnostic;
use proto_core::{is_offline, now, ProtoEnvironment};
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
pub fn detect_proto_env() -> AppResult<ProtoEnvironment> {
    ProtoEnvironment::new()
}

// ANALYZE

#[instrument(skip_all)]
pub fn sync_proto_tool(env: &ProtoEnvironment, version: &str) -> AppResult {
    let tool_dir = env.store.inventory_dir.join("proto").join(version);
    let exe_names = vec![get_exe_file_name("proto"), get_exe_file_name("proto-shim")];

    for exe_name in exe_names {
        let src_file = env.store.bin_dir.join(&exe_name);
        let dst_file = tool_dir.join(&exe_name);

        if src_file.exists() && !dst_file.exists() {
            fs::copy_file(src_file, dst_file)?;
        }
    }

    Ok(())
}

#[instrument(skip_all)]
pub fn load_proto_configs(env: &ProtoEnvironment) -> AppResult {
    env.load_config()?;

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
