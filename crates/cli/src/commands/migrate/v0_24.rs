#![allow(deprecated)]

use crate::helpers::ProtoResource;
use proto_core::ProtoConfig;
use starbase::SystemResult;
use starbase_styles::color;
use std::mem;
use tracing::{debug, info};

pub async fn migrate(proto: &ProtoResource) -> SystemResult {
    info!("Loading tools...");

    let tools = proto.load_tools().await?;
    let mut config = ProtoConfig::load_from(proto.env.get_config_dir(true), false)?;
    let mut updated_config = false;

    info!("Migrating configs...");

    for tool in tools {
        debug!("Checking {}", color::id(&tool.id));

        let mut manifest = tool.manifest.clone();
        let mut updated_manifest = false;

        if !manifest.aliases.is_empty() {
            debug!("Found aliases");

            let tool_config = config
                .tools
                .get_or_insert(Default::default())
                .entry(tool.id.clone())
                .or_default();

            tool_config
                .aliases
                .get_or_insert(Default::default())
                .extend(mem::take(&mut manifest.aliases));

            updated_manifest = true;
        }

        if manifest.default_version.is_some() {
            debug!("Found default version");

            config
                .versions
                .get_or_insert(Default::default())
                .insert(tool.id.clone(), manifest.default_version.take().unwrap());

            updated_manifest = true;
        }

        if updated_manifest {
            updated_config = true;
            manifest.save()?;
        }
    }

    if updated_config {
        ProtoConfig::save_to(proto.env.get_config_dir(true), config)?;
    }

    info!("Migration complete!");

    Ok(())
}
