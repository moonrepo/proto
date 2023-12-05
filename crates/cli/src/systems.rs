#![allow(deprecated)]

use crate::helpers::ProtoResource;
use proto_core::{Id, ProtoConfig, UserConfig, PROTO_CONFIG_NAME, USER_CONFIG_NAME};
use starbase::system;
use starbase_utils::fs;
use starbase_utils::json::JsonValue;
use tracing::debug;

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
