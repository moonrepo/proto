use std::env;

use crate::shell;
use clap_complete::Shell;
use log::info;
use proto_core::{color, ProtoError};
use rustc_hash::FxHashMap;

pub fn post_install(passthrough: &[String]) -> Result<(), ProtoError> {
    if passthrough.contains(&"--no-gobin".to_string()) || env::var("GOBIN").is_ok() {
        return Ok(());
    }

    let Some(shell) = Shell::from_env() else {
        return Ok(());
    };

    let env_vars = FxHashMap::from_iter([
        ("GOBIN".to_string(), "$HOME/go/bin".to_string()),
        ("PATH".to_string(), "$GOBIN".to_string()),
    ]);

    if let Some(content) = shell::format_env_vars(&shell, "go", env_vars) {
        if let Some(updated_profile) = shell::write_profile_if_not_setup(&shell, content, "GOBIN")?
        {
            info!(
                target: "proto:install", "Added GOBIN to your shell profile {}",
                color::path(updated_profile)
            );
        }
    }

    Ok(())
}
