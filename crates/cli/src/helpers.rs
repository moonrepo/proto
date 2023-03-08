#![allow(clippy::borrowed_box)]

use crate::config::{Config, CONFIG_NAME};
use log::{debug, trace};
use proto::{color, get_tools_dir, load_version_file, ProtoError, Tool, ToolType};
use std::{
    env,
    path::{Path, PathBuf},
};

pub fn enable_logging() {
    if let Ok(level) = env::var("PROTO_LOG") {
        if !level.starts_with("proto=") {
            env::set_var("PROTO_LOG", format!("proto={level}"));
        }
    } else {
        env::set_var("PROTO_LOG", "proto=info");
    }

    env_logger::Builder::from_env("PROTO_LOG")
        .format_timestamp(None)
        .init();
}

pub fn get_global_version_path(tool: &Box<dyn Tool<'_>>) -> Result<PathBuf, ProtoError> {
    Ok(get_tools_dir()?.join(tool.get_bin_name()).join("version"))
}

pub async fn detect_version_from_environment(
    tool: &Box<dyn Tool<'_>>,
    tool_type: &ToolType,
    forced_version: Option<String>,
) -> Result<String, ProtoError> {
    let mut version = forced_version;
    let env_var = format!("PROTO_{}_VERSION", tool.get_bin_name().to_uppercase());

    // Env var takes highest priority
    if version.is_none() {
        if let Ok(session_version) = env::var(&env_var) {
            debug!(
                target: "proto:detect",
                "Detected version {} from environment variable {}",
                session_version,
                env_var
            );

            version = Some(session_version);
        }
    } else {
        debug!(
            target: "proto:detect",
            "Using explicit version {} passed on the command line",
            version.as_ref().unwrap(),
        );
    }

    // Traverse upwards and attempt to detect a local version
    if let Ok(working_dir) = env::current_dir() {
        trace!(
            target: "proto:detect",
            "Attempting to find local version"
        );

        let mut current_dir: Option<&Path> = Some(&working_dir);

        while let Some(dir) = &current_dir {
            trace!(
                target: "proto:detect",
                "Checking in directory {}",
                color::path(dir)
            );

            // We already found a version, so exit
            if version.is_some() {
                break;
            }

            // Detect from our config file
            trace!(
                target: "proto:detect",
                "Checking proto configuration file"
            );

            let config_file = dir.join(CONFIG_NAME);

            if config_file.exists() {
                let config = Config::load(&config_file)?;

                if let Some(config_version) = config.tools.get(tool_type) {
                    debug!(
                        target: "proto:detect",
                        "Detected version {} from configuration file {}",
                        config_version,
                        color::path(&config_file)
                    );

                    version = Some(config_version.to_owned());
                    break;
                }
            }

            // Detect using the tool
            trace!(
                target: "proto:detect",
                "Detecting from the tool's ecosystem"
            );

            if let Some(eco_version) = tool.detect_version_from(dir).await? {
                debug!(
                    target: "proto:detect",
                    "Detected version {} from tool's ecosystem",
                    eco_version,
                );

                version = Some(eco_version);
                break;
            }

            current_dir = dir.parent();
        }
    }

    // If still no version, load the global version
    if version.is_none() {
        trace!(
            target: "proto:detect",
            "Attempting to find global version"
        );

        let global_file = get_global_version_path(tool)?;

        if global_file.exists() {
            let global_version = load_version_file(&global_file)?;

            debug!(
                target: "proto:detect",
                "Detected global version {} from {}",
                global_version,
                color::path(&global_file)
            );

            version = Some(global_version);
        }
    }

    // We didn't find anything!
    match version {
        Some(ver) => Ok(ver),
        None => Err(ProtoError::Message(
            "Unable to detect an applicable version. Try setting a local or global version, or passing a command line argument.".into(),
        )),
    }
}
