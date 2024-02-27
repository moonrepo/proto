#![allow(deprecated)]

use crate::helpers::{fetch_latest_version, ProtoResource};
use miette::IntoDiagnostic;
use proto_core::{is_offline, now};
use semver::Version;
use starbase::system;
use starbase_styles::color;
use starbase_utils::fs;
use std::env;
use std::time::Duration;
use tracing::debug;

// STARTUP

#[system]
pub fn detect_proto_env(resources: ResourcesMut) {
    resources.set(ProtoResource::new()?);
}

// ANALYZE

#[system]
pub fn load_proto_configs(proto: ResourceMut<ProtoResource>) {
    proto.env.load_config()?;
}

#[system]
pub fn remove_old_bins(proto: ResourceRef<ProtoResource>) {
    // These bins are no longer supported but we don't have an easy
    // way to "clean up" bins that are no longer configured in a plugin.
    for bin in ["npm", "npx", "node-gyp", "pnpm", "pnpx", "yarn", "yarnpkg"] {
        let _ = fs::remove_file(proto.env.bin_dir.join(if cfg!(windows) {
            format!("{bin}.cmd")
        } else {
            bin.to_owned()
        }));
    }
}

// EXECUTE

#[system]
pub async fn check_for_new_version(proto: ResourceRef<ProtoResource>) {
    if
    // Don't check when running tests
    env::var("PROTO_TEST").is_ok() ||
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
    let cache_file = proto.env.temp_dir.join(".last-version-check");

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
}
