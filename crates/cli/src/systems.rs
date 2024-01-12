#![allow(deprecated)]

use crate::commands::fetch_version;
use crate::helpers::ProtoResource;
use miette::IntoDiagnostic;
use proto_core::{is_offline, Id, ProtoConfig, UserConfig, PROTO_CONFIG_NAME, USER_CONFIG_NAME};
use semver::Version;
use starbase::system;
use starbase_styles::color;
use starbase_utils::fs;
use starbase_utils::json::JsonValue;
use std::env;
use tracing::{debug, info};

// STARTUP

#[system]
pub fn detect_proto_env(resources: ResourcesMut) {
    resources.set(ProtoResource::new()?);
}

#[system]
pub fn migrate_user_config(proto: ResourceRef<ProtoResource>) {
    let dir = proto.env.get_config_dir(true);
    let old_file = dir.join(USER_CONFIG_NAME);

    if !old_file.exists() {
        return Ok(());
    }

    let new_file = dir.join(PROTO_CONFIG_NAME);

    debug!(
        old_file = ?old_file,
        new_file = ?new_file,
        "Detected legacy user config file, migrating to new format",
    );

    let user_config = UserConfig::load_from(dir)?;

    ProtoConfig::update(dir, |config| {
        let settings = config.settings.get_or_insert(Default::default());
        settings.auto_clean = user_config.auto_clean;
        settings.auto_install = user_config.auto_install;
        settings.detect_strategy = user_config.detect_strategy;
        settings.http = user_config.http;
        settings.pin_latest = user_config.pin_latest;

        if !user_config.plugins.is_empty() {
            let plugins = config.plugins.get_or_insert(Default::default());
            plugins.extend(user_config.plugins);
        }

        if let Some(node_intercept_globals) = user_config.node_intercept_globals {
            let tools = config.tools.get_or_insert(Default::default());
            let node_config = tools.entry(Id::raw("node")).or_default();

            node_config.config.get_or_insert(Default::default()).insert(
                "intercept-globals".into(),
                JsonValue::Bool(node_intercept_globals),
            );
        }
    })?;

    debug!(file = ?old_file, "Migrated, deleting legacy user config file");

    fs::remove_file(old_file)?;
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

#[system(instrument = false)]
pub async fn check_for_new_version() {
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

    let current_version = env!("CARGO_PKG_VERSION");

    debug!(current_version, "Checking for a new version of proto");

    let Ok(latest_version) = fetch_version().await else {
        return Ok(());
    };

    let local_version = Version::parse(current_version).into_diagnostic()?;
    let remote_version = Version::parse(&latest_version).into_diagnostic()?;
    let update_available = remote_version != local_version;

    if update_available {
        debug!(latest_version = &latest_version, "Found a newer version");

        info!(
            "There's a new version of proto available, {} (currently on {})!",
            color::hash(remote_version.to_string()),
            color::muted_light(local_version.to_string()),
        );

        info!(
            "Run {} or install from {}",
            color::shell("proto upgrade"),
            color::url("https://moonrepo.dev/docs/proto/install"),
        );
    }
}
