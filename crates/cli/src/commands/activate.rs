use crate::session::{LoadToolOptions, ProtoSession};
use crate::workflows::{ExecWorkflow, ExecWorkflowParams};
use clap::Args;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::{Id, PROTO_PLUGIN_KEY, ToolContext, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use serde::Serialize;
use starbase::AppResult;
use starbase_shell::{Hook, ShellType};
use starbase_utils::json;
use std::env;
use tracing::warn;

#[derive(Serialize)]
struct ActivateResult {
    env: IndexMap<String, Option<String>>,
    path: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ActivateArgs {
    #[arg(help = "Shell to activate for")]
    shell: Option<ShellType>,

    #[arg(
        long,
        help = "Print the activate instructions in shell specific-syntax"
    )]
    export: bool,

    #[arg(long, help = "Don't include ~/.proto/bin in path lookup")]
    no_bin: bool,

    #[arg(long, help = "Do not run activate hook on initialization")]
    no_init: bool,

    #[arg(long, help = "Don't include ~/.proto/shims in path lookup")]
    no_shim: bool,

    #[arg(long, help = "Run activate hook on initialization and export")]
    on_init: bool,
}

#[tracing::instrument(skip_all)]
pub async fn activate(session: ProtoSession, args: ActivateArgs) -> AppResult {
    // Detect the shell that we need to activate for
    let shell_type = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    // If not exporting data, just print the activation syntax immediately
    if !args.export && !session.should_print_json() {
        print_activation_hook(&session, &shell_type, &args)?;

        return Ok(None);
    }

    // Load configuration and tools
    let config = session.env.load_config()?;
    let tools = session
        .load_tools_with_options(LoadToolOptions {
            detect_version: true,
            ..Default::default()
        })
        .await?;

    // Extract specs for each tool
    let mut specs = FxHashMap::default();

    for tool in &tools {
        if let Some(spec) = &tool.detected_version {
            specs.insert(tool.context.clone(), spec.to_owned());
        }
    }

    // Aggregate our environment/shell exports
    let mut workflow = ExecWorkflow::new(tools, config);

    workflow
        .prepare_environment(
            specs,
            ExecWorkflowParams {
                activate_environment: true,
                ..Default::default()
            },
        )
        .await?;

    // Inject necessary variables
    if !workflow.env.contains_key("PROTO_HOME") && env::var("PROTO_HOME").is_err() {
        workflow.env.insert(
            "PROTO_HOME".into(),
            session.env.store.dir.to_str().map(|root| root.to_owned()),
        );
    }

    let proto_context = ToolContext::new(Id::raw(PROTO_PLUGIN_KEY));

    if let Some(UnresolvedVersionSpec::Semantic(version)) =
        config.versions.get(&proto_context).map(|spec| &spec.req)
    {
        workflow
            .env
            .insert("PROTO_VERSION".into(), Some(version.to_string()));
        workflow
            .env
            .insert("PROTO_PROTO_VERSION".into(), Some(version.to_string()));

        workflow.paths.push_back(
            session
                .env
                .store
                .inventory_dir
                .join("proto")
                .join(version.to_string()),
        );
    } else {
        workflow.env.insert("PROTO_VERSION".into(), None);
    }

    if !args.no_shim {
        workflow
            .paths
            .push_back(session.env.store.shims_dir.clone());
    }

    if !args.no_bin {
        workflow.paths.push_back(session.env.store.bin_dir.clone());
    }

    // Output/export the information for the chosen shell
    if args.export {
        print_activation_exports(&session, &shell_type, workflow)?;

        return Ok(None);
    }

    if session.should_print_json() {
        let result = ActivateResult {
            path: workflow
                .reset_and_join_paths(&session.env.store.dir)?
                .into_string()
                .ok(),
            env: workflow.env,
        };

        session
            .console
            .out
            .write_line(json::format(&result, true)?)?;
    }

    Ok(None)
}

fn print_activation_hook(
    session: &ProtoSession,
    shell_type: &ShellType,
    args: &ActivateArgs,
) -> miette::Result<()> {
    let mut command = format!("proto activate {shell_type}");

    if let Some(mode) = &session.cli.config_mode {
        command.push_str(" --config-mode ");
        command.push_str(&mode.to_string());
    }

    if args.no_bin {
        command.push_str(" --no-bin");
    }

    if args.no_shim {
        command.push_str(" --no-shim");
    }

    match shell_type {
        // These operate on JSON
        ShellType::Nu => {
            command.push_str(" --json");
        }
        // While these evaluate shell syntax
        _ => {
            command.push_str(" --export");
        }
    };

    if args.on_init {
        warn!(
            "The --on-init option is deprecated and can be removed. This functionality is now the default."
        );
    }

    session
        .console
        .out
        .write_line(shell_type.build().format_hook(Hook::OnChangeDir {
            command,
            function: "_proto_activate_hook".into(),
        })?)?;

    if !args.no_init {
        session.console.out.write_line("\n_proto_activate_hook")?;
    }

    Ok(())
}

fn print_activation_exports(
    session: &ProtoSession,
    shell_type: &ShellType,
    workflow: ExecWorkflow,
) -> miette::Result<()> {
    let shell = shell_type.build();
    let mut env_being_set = vec![];
    let mut output = vec![];

    // Remove previously set variables
    if let Ok(env_to_remove) = env::var("_PROTO_ACTIVATED_ENV") {
        for key in env_to_remove.split(',') {
            if !workflow.env.contains_key(key) {
                output.push(shell.format_env_unset(key));
            }
        }
    }

    // Set/remove new variables
    for (key, value) in &workflow.env {
        if value.is_some() {
            env_being_set.push(key.to_owned());
        }

        output.push(shell.format_env(key, value.as_deref()));
    }

    if !env_being_set.is_empty() {
        output.push(shell.format_env_set("_PROTO_ACTIVATED_ENV", &env_being_set.join(",")));
    }

    // Set new `PATH`
    if !workflow.paths.is_empty() {
        output.push(
            shell.format_env_set(
                "_PROTO_ACTIVATED_PATH",
                env::join_paths(&workflow.paths)
                    .into_diagnostic()?
                    .to_str()
                    .unwrap_or_default(),
            ),
        );

        let paths = workflow
            .reset_paths(&session.env.store.dir)
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        if !paths.is_empty() {
            output.push(shell.format_path_set(&paths));
        }
    }

    session.console.out.write_line(output.join("\n"))?;

    Ok(())
}
