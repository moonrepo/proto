#![allow(deprecated)]

use crate::helpers::load_configured_tools;
use proto_core::UserConfig;
use starbase::SystemResult;
use starbase_styles::color;
use std::mem;
use tracing::{debug, info};

pub async fn migrate() -> SystemResult {
    info!("Loading tools...");

    let tools = load_configured_tools().await?;
    let mut user_config = UserConfig::load()?;
    let mut updated_user_config = false;

    info!("Migrating configs...");

    for tool in tools {
        debug!("Checking {}", color::id(&tool.id));

        let mut manifest = tool.manifest.clone();
        let mut updated_manifest = false;

        if !manifest.aliases.is_empty() {
            debug!("Found aliases");

            let entry = user_config.tools.entry(tool.id.clone()).or_default();
            entry.aliases.extend(mem::take(&mut manifest.aliases));
            updated_manifest = true;
        }

        if !tool.manifest.default_version.is_some() {
            debug!("Found a default version");

            let entry = user_config.tools.entry(tool.id.clone()).or_default();
            entry.default_version = manifest.default_version.take();
            updated_manifest = true;
        }

        if updated_manifest {
            updated_user_config = true;
            manifest.save()?;
        }
    }

    if updated_user_config {
        user_config.save()?;
    }

    info!("Migration complete!");

    Ok(())
}
