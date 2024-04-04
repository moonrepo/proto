use super::clean::clean_plugins;
use super::pin::internal_pin;
use crate::helpers::{create_progress_bar, disable_progress_bars, ProtoResource};
use crate::shell::{self, Export};
use crate::telemetry::{track_usage, Metric};
use clap::{Args, ValueEnum};
use proto_core::{Id, PinType, Tool, UnresolvedVersionSpec};
use proto_pdk_api::{InstallHook, SyncShellProfileInput, SyncShellProfileOutput};
use starbase::system;
use starbase_styles::color;
use std::env;
use tracing::debug;

#[derive(Clone, Debug, ValueEnum)]
pub enum PinOption {
    Global,
    Local,
}

#[derive(Args, Clone, Debug)]
pub struct InstallArgs {
    #[arg(required = true, help = "ID of tool")]
    pub id: Id,

    #[arg(
        default_value = "latest",
        help = "Version or alias of tool",
        group = "version-type"
    )]
    pub spec: Option<UnresolvedVersionSpec>,

    #[arg(
        long,
        help = "Install a canary (nightly, etc) version",
        group = "version-type"
    )]
    pub canary: bool,

    #[arg(long, help = "Pin the resolved version to .prototools")]
    pub pin: Option<Option<PinOption>>,

    // Passthrough args (after --)
    #[arg(last = true, help = "Unique arguments to pass to each tool")]
    pub passthrough: Vec<String>,
}

async fn pin_version(
    tool: &mut Tool,
    initial_version: &UnresolvedVersionSpec,
    arg_pin_type: &Option<PinType>,
) -> miette::Result<bool> {
    let config = tool.proto.load_config()?;
    let spec = tool.get_resolved_version().to_unresolved_spec();
    let mut global = false;
    let mut pin = false;

    // via `--pin` arg
    if let Some(pin_type) = arg_pin_type {
        global = matches!(pin_type, PinType::Global);
        pin = true;
    }
    // Or the first time being installed
    else if !config.versions.contains_key(&tool.id) {
        global = true;
        pin = true;
    }

    // via `pin-latest` setting
    if initial_version.is_latest() {
        if let Some(pin_type) = &config.settings.pin_latest {
            global = matches!(pin_type, PinType::Global);
            pin = true;
        }
    }

    if pin {
        internal_pin(tool, &spec, global, true).await?;
    }

    Ok(pin)
}

pub async fn internal_install(
    proto: &ProtoResource,
    args: InstallArgs,
    tool: Option<Tool>,
) -> miette::Result<Tool> {
    let mut tool = match tool {
        Some(tool) => tool,
        None => proto.load_tool(&args.id).await?,
    };

    let version = if args.canary {
        UnresolvedVersionSpec::Canary
    } else {
        args.spec.clone().unwrap_or_default()
    };

    let pin_type = args.pin.map(|pin| match pin {
        Some(PinOption::Global) => PinType::Global,
        _ => PinType::Local,
    });

    // Disable version caching and always use the latest when installing
    tool.disable_caching();

    if tool.disable_progress_bars() {
        disable_progress_bars();
    }

    // Resolve version first so subsequent steps can reference the resolved version
    tool.resolve_version(&version, false).await?;

    // Check if already installed, or if canary, overwrite previous install
    if !version.is_canary() && tool.is_setup(&version).await? {
        pin_version(&mut tool, &version, &pin_type).await?;

        println!(
            "{} {} has already been installed at {}",
            tool.get_name(),
            tool.get_resolved_version(),
            color::path(tool.get_product_dir()),
        );

        return Ok(tool);
    }

    let resolved_version = tool.get_resolved_version();

    // This ensures that the correct version is used by other processes
    env::set_var(
        format!("{}_VERSION", tool.get_env_var_prefix()),
        resolved_version.to_string(),
    );

    env::set_var("PROTO_INSTALL", args.id.to_string());

    // Run before hook
    if tool.plugin.has_func("pre_install") {
        tool.plugin.call_func_without_output(
            "pre_install",
            InstallHook {
                context: tool.create_context(),
                passthrough_args: args.passthrough.clone(),
                pinned: pin_type.is_some(),
            },
        )?;
    }

    // Install the tool
    debug!(
        "Installing {} with version {} (from {})",
        tool.get_name(),
        resolved_version,
        version
    );

    let pb = create_progress_bar(format!(
        "Installing {} {}",
        tool.get_name(),
        resolved_version
    ));

    let installed = tool.setup(&version, false).await?;

    pb.finish_and_clear();

    if !installed {
        return Ok(tool);
    }

    let pinned = pin_version(&mut tool, &version, &pin_type).await?;

    println!(
        "{} {} has been installed to {}!",
        tool.get_name(),
        tool.get_resolved_version(),
        color::path(tool.get_product_dir()),
    );

    // Track usage metrics
    track_usage(
        &tool.proto,
        Metric::InstallTool {
            id: tool.id.to_string(),
            plugin: tool
                .locator
                .as_ref()
                .map(|loc| loc.to_string())
                .unwrap_or_default(),
            version: resolved_version.to_string(),
            version_candidate: version.to_string(),
            pinned,
        },
    )
    .await?;

    // Run after hook
    if tool.plugin.has_func("post_install") {
        tool.plugin.call_func_without_output(
            "post_install",
            InstallHook {
                context: tool.create_context(),
                passthrough_args: args.passthrough.clone(),
                pinned: pin_type.is_some(),
            },
        )?;
    }

    // Sync shell profile
    update_shell(&tool, args.passthrough.clone())?;

    // Clean plugins
    debug!("Auto-cleaning plugins");

    clean_plugins(proto, 7).await?;

    Ok(tool)
}

fn update_shell(tool: &Tool, passthrough_args: Vec<String>) -> miette::Result<()> {
    if !tool.plugin.has_func("sync_shell_profile") {
        return Ok(());
    }

    let output: SyncShellProfileOutput = tool.plugin.call_func_with(
        "sync_shell_profile",
        SyncShellProfileInput {
            context: tool.create_context(),
            passthrough_args,
        },
    )?;

    if output.skip_sync {
        return Ok(());
    }

    let shell_type = shell::detect_shell(None);

    debug!(
        shell = ?shell_type,
        env_vars = ?output.export_vars,
        paths = ?output.extend_path,
        "Updating shell profile",
    );

    let mut exports = vec![];

    if let Some(export_vars) = output.export_vars {
        for (key, value) in export_vars {
            exports.push(Export::Var(key, value));
        }
    }

    if let Some(extend_path) = output.extend_path {
        exports.push(Export::Path(extend_path));
    }

    if let Some(content) = shell::format_exports(&shell_type, tool.id.as_str(), exports) {
        let profile_path = tool.proto.store.load_preferred_profile()?.and_then(|file| {
            if file.exists() {
                Some(file)
            } else {
                debug!(
                    profile = ?file,
                    "Configured shell profile path does not exist, will not use",
                );
                None
            }
        });

        let updated_profile = match profile_path {
            Some(profile_path) => Some(shell::write_profile(
                &profile_path,
                &content,
                &output.check_var,
            )?),
            None => shell::write_profile_if_not_setup(&shell_type, &content, &output.check_var)?,
        };

        if let Some(updated_profile) = updated_profile {
            println!(
                "Added {} to shell profile {}",
                color::property(output.check_var),
                color::path(updated_profile)
            );
        }
    }

    Ok(())
}

#[system]
pub async fn install(args: ArgsRef<InstallArgs>, proto: ResourceRef<ProtoResource>) {
    internal_install(proto, args.to_owned(), None).await?;
}
