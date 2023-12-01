#![allow(deprecated)]

use crate::helpers::ProtoResource;
use crate::shell;
use proto_core::get_bin_dir;
use starbase::SystemResult;
use starbase_utils::fs;
use tracing::{debug, info};

pub async fn migrate(proto: &ProtoResource) -> SystemResult {
    info!("Loading tools...");

    let tools = proto.load_tools().await?;

    // Skips tools/plugins that are not in use
    let mut tools = tools
        .into_iter()
        .filter(|tool| !tool.manifest.installed_versions.is_empty())
        .collect::<Vec<_>>();

    for tool in &mut tools {
        // Resolve the global version for use in shims and bins
        if let Some(spec) = tool.manifest.default_version.clone() {
            tool.resolve_version(&spec, false).await?;
        }
    }

    info!("Deleting old shims...");

    for file in fs::read_dir(get_bin_dir()?)? {
        let path = file.path();
        let name = fs::file_name(&path);

        if name == "proto" || name == "proto.exe" || name == "moon" || name == "moon.exe" {
            continue;
        }

        debug!(shim = ?path, "Deleting shim");

        fs::remove_file(path)?;
    }

    info!("Generating new shims...");

    for tool in &mut tools {
        // Always create shims for all active tools
        tool.generate_shims(true).await?;
    }

    info!("Linking new binaries...");

    for tool in &mut tools {
        // Only the global version is linked, so only create if set
        if tool.manifest.default_version.is_some() {
            tool.symlink_bins(true).await?;
        }
    }

    update_shell()?;

    info!("Migration complete!");

    Ok(())
}

#[cfg(not(windows))]
fn update_shell() -> SystemResult {
    use crate::shell::format_env_var;

    info!("Updating shell profile...");

    let shell = shell::detect_shell(None);
    let substitutions = vec![
        (
            format_env_var(&shell, "PROTO_ROOT", "$HOME/.proto").unwrap(),
            format_env_var(&shell, "PROTO_HOME", "$HOME/.proto").unwrap(),
        ),
        (
            format_env_var(&shell, "PATH", "$PROTO_ROOT/bin").unwrap(),
            format_env_var(&shell, "PATH", "$PROTO_HOME/shims:$PROTO_HOME/bin").unwrap(),
        ),
        (
            format_env_var(&shell, "PATH", "$PROTO_HOME/bin").unwrap(),
            format_env_var(&shell, "PATH", "$PROTO_HOME/shims:$PROTO_HOME/bin").unwrap(),
        ),
    ];

    for profile_path in shell::find_profiles(&shell)? {
        if !profile_path.exists() {
            continue;
        }

        let mut profile = fs::read_file(&profile_path)?;
        let mut modified = false;

        for (find, replace) in &substitutions {
            if profile.contains(find) {
                profile = profile.replace(find, replace);
                modified = true;

                debug!(
                    profile = ?profile_path,
                    old = find,
                    new = replace,
                    "Replacing environment variable",
                );
            }
        }

        if modified {
            fs::write_file(profile_path, profile)?;
        }
    }

    Ok(())
}

#[cfg(windows)]
fn update_shell() -> SystemResult {
    use crate::commands::setup::do_setup;
    use proto_core::get_shims_dir;
    use tracing::warn;

    info!("Updating environment variables...");

    do_setup(
        shell::detect_shell(None),
        vec![get_shims_dir()?, get_bin_dir()?],
        false,
    )?;

    warn!("Audit your system variables to ensure they're correct!");

    Ok(())
}
