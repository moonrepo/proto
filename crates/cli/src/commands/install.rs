use super::pin::internal_pin;
use crate::helpers::{create_progress_bar, disable_progress_bars, enable_progress_bars};
use crate::session::ProtoSession;
use crate::shell::{self, Export};
use crate::telemetry::{track_usage, Metric};
use clap::{Args, ValueEnum};
use miette::IntoDiagnostic;
use proto_core::{Id, PinType, Tool, UnresolvedVersionSpec};
use proto_pdk_api::{InstallHook, SyncShellProfileInput, SyncShellProfileOutput};
use starbase::AppResult;
use starbase_shell::ShellType;
use starbase_styles::color;
use std::env;
use std::process;
use tracing::debug;

#[derive(Clone, Debug, ValueEnum)]
pub enum PinOption {
    Global,
    Local,
}

#[derive(Args, Clone, Debug, Default)]
pub struct InstallArgs {
    // ONE
    #[arg(help = "ID of a single tool to install")]
    pub id: Option<Id>,

    #[arg(
        default_value = "latest",
        help = "When installing one, the version or alias to install",
        group = "version-type"
    )]
    pub spec: Option<UnresolvedVersionSpec>,

    #[arg(
        long,
        help = "When installing one, use a canary (nightly, etc) version",
        group = "version-type"
    )]
    pub canary: bool,

    #[arg(
        long,
        help = "When installing one, pin the resolved version to .prototools"
    )]
    pub pin: Option<Option<PinOption>>,

    // ALL
    #[arg(
        long,
        help = "When installing all, include versions configured in global ~/.proto/.prototools"
    )]
    pub include_global: bool,

    // Passthrough args (after --)
    #[arg(
        last = true,
        help = "When installing one, additional arguments to pass to the tool"
    )]
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

    let shell_type = ShellType::try_detect()?;
    let shell = shell_type.build();

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

    if exports.is_empty() {
        return Ok(());
    }

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

    if let Some(updated_profile) = profile_path {
        let exported_content = shell::format_exports(&shell, &tool.id, exports);

        if shell::update_profile_if_not_setup(
            &updated_profile,
            &exported_content,
            &output.check_var,
        )? {
            println!(
                "Added {} to shell profile {}",
                color::property(output.check_var),
                color::path(updated_profile)
            );
        }
    }

    Ok(())
}

#[tracing::instrument(skip(session, args, tool))]
pub async fn install_one(
    session: &ProtoSession,
    args: InstallArgs,
    id: &Id,
    tool: Option<Tool>,
) -> miette::Result<Tool> {
    debug!(id = id.as_str(), "Loading tool");

    let mut tool = match tool {
        Some(tool) => tool,
        None => session.load_tool(id).await?,
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

    env::set_var("PROTO_INSTALL", id.to_string());

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

    Ok(tool)
}

#[tracing::instrument(skip_all)]
pub async fn install_all(session: &ProtoSession, args: InstallArgs) -> AppResult {
    debug!("Loading all tools");

    let tools = session.load_tools().await?;

    debug!("Detecting tool versions to install");

    let config = if args.include_global {
        session.env.load_config_manager()?.get_merged_config()?
    } else {
        session
            .env
            .load_config_manager()?
            .get_merged_config_without_global()?
    };
    let mut versions = config.versions.to_owned();
    versions.remove("proto");

    for tool in &tools {
        if versions.contains_key(&tool.id) {
            continue;
        }

        if let Some((candidate, _)) = tool.detect_version_from(&session.env.cwd).await? {
            debug!("Detected version {} for {}", candidate, tool.get_name());

            versions.insert(tool.id.clone(), candidate);
        }
    }

    if versions.is_empty() {
        eprintln!("No versions have been configured, nothing to install!");

        process::exit(1);
    }

    let pb = create_progress_bar(format!(
        "Installing {} tools: {}",
        versions.len(),
        versions
            .keys()
            .map(color::id)
            .collect::<Vec<_>>()
            .join(", ")
    ));

    disable_progress_bars();

    // Then install each tool in parallel!
    let mut futures = vec![];

    for tool in tools {
        if let Some(version) = versions.remove(&tool.id) {
            let proto_clone = session.clone();
            let id = tool.id.clone();

            futures.push(tokio::spawn(async move {
                install_one(
                    &proto_clone,
                    InstallArgs {
                        spec: Some(version),
                        ..Default::default()
                    },
                    &id,
                    Some(tool),
                )
                .await
            }));
        }
    }

    for future in futures {
        future.await.into_diagnostic()??;
    }

    enable_progress_bars();

    pb.finish_and_clear();

    println!("Successfully installed tools!");

    Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn install(session: ProtoSession, args: InstallArgs) -> AppResult {
    match args.id.clone() {
        Some(id) => {
            install_one(&session, args, &id, None).await?;
        }
        None => {
            install_all(&session, args).await?;
        }
    };

    Ok(())
}
