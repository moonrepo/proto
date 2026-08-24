use crate::app::StdoutOwner;
use crate::session::{LoadToolOptions, ProtoSession, SessionResult};
use crate::workflows::{ExecWorkflow, ExecWorkflowParams};
use clap::Args;
use indexmap::IndexMap;
use proto_core::{Id, PROTO_PLUGIN_KEY, ToolContext, UnresolvedVersionSpec};
use rustc_hash::FxHashMap;
use serde::Serialize;
use starbase_shell::{Hook, ShellType};
use starbase_utils::envx::is_test;
use std::env;
use std::path::PathBuf;
use tracing::instrument;

/// Environment variables that track what the previous activation applied,
/// so that a follow-up activation (or `proto deactivate`) can reverse it.
pub const ACTIVATED_ALIASES_KEY: &str = "_PROTO_ACTIVATED_ALIASES";
pub const ACTIVATED_ENV_KEY: &str = "_PROTO_ACTIVATED_ENV";
pub const ACTIVATED_PATH_KEY: &str = "_PROTO_ACTIVATED_PATH";

/// The payload that both `proto activate` and `proto deactivate` print in
/// structured mode. The shape is a contract with the nu hook, which cannot
/// evaluate shell syntax, so both commands must serialize the same fields.
///
/// `PATH` is a list and not a pre-joined string, as nu models it as a list,
/// and would otherwise lose its path semantics.
#[derive(Serialize)]
pub struct ActivateOutput {
    pub env: IndexMap<String, Option<String>>,
    pub paths: Vec<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ActivateArgs {
    #[arg(help = "Shell to activate for")]
    shell: Option<ShellType>,

    #[arg(
        long,
        help = "Print the activate instructions in shell specific-syntax"
    )]
    pub export: bool,

    #[arg(long, help = "Don't include ~/.proto/bin in path lookup")]
    no_bin: bool,

    #[arg(long, help = "Do not run activate hook on initialization")]
    no_init: bool,

    #[arg(long, help = "Don't include ~/.proto/shims in path lookup")]
    no_shim: bool,
}

#[derive(PartialEq)]
enum ActivateOutputMode {
    Hook,
    Export,
    Structured,
}

#[instrument(skip(session))]
pub async fn activate(session: ProtoSession, args: ActivateArgs) -> SessionResult {
    // Detect the shell that we need to activate for
    let shell_type = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    // Shell code is this command's default protocol; see `App::stdout_owner`
    let output_mode = match session.cli.stdout_owner() {
        StdoutOwner::Reporter => ActivateOutputMode::Structured,
        StdoutOwner::ShellCode if args.export => ActivateOutputMode::Export,
        StdoutOwner::ShellCode => ActivateOutputMode::Hook,
        StdoutOwner::CompletionCode | StdoutOwner::McpStdio => {
            unreachable!("activate resolved to an unrelated stdout owner")
        }
    };

    // Hook mode does not need to load tools or build an environment.
    if output_mode == ActivateOutputMode::Hook {
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

    if let Some(UnresolvedVersionSpec::Version(version)) =
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
    if output_mode == ActivateOutputMode::Export {
        print_activation_exports(&session, &shell_type, workflow)?;

        return Ok(None);
    }

    session.console.write_json_for_format(ActivateOutput {
        env: create_activation_env(&workflow, &shell_type)?,
        paths: stringify_paths(workflow.reset_paths_for_shell(&session.env.store.dir, &shell_type)),
    })?;

    Ok(None)
}

fn print_activation_hook(
    session: &ProtoSession,
    shell_type: &ShellType,
    args: &ActivateArgs,
) -> miette::Result<()> {
    let mut activate_command = format!("proto activate {shell_type}");
    let mut deactivate_command = format!("proto deactivate {shell_type}");

    // Deactivating reverses what was applied to the shell session, and
    // never loads configuration or tools, so it inherits no other args.
    if let Some(mode) = &session.cli.config_mode {
        activate_command.push_str(" --config-mode ");
        activate_command.push_str(&mode.to_string());
    }

    if args.no_bin {
        activate_command.push_str(" --no-bin");
    }

    if args.no_shim {
        activate_command.push_str(" --no-shim");
    }

    let output_arg = match shell_type {
        // These operate on JSON
        ShellType::Nu => " --reporter json",
        // While these evaluate shell syntax
        _ => " --export",
    };

    activate_command.push_str(output_arg);
    deactivate_command.push_str(output_arg);

    session
        .console
        .out
        .write_line(shell_type.build().format_hook(Hook::OnChangeDir {
            activate_command,
            activate_function: "proto_activate".into(),
            deactivate_command,
            deactivate_function: "proto_deactivate".into(),
        })?)?;

    if !args.no_init {
        match shell_type {
            // Nu modules can only contain definitions, so a call here would
            // make the module fail to parse. Its `export-env` block registers
            // the hook, and it can be called manually after the module is used.
            ShellType::Nu => {}
            // Parens are required for xonsh as it is Python-based
            ShellType::Xonsh => {
                session.console.out.write_line("\nproto_activate()")?;
            }
            // While others are shell scripts
            _ => {
                session.console.out.write_line("\nproto_activate")?;
            }
        };
    }

    Ok(())
}

/// Create the environment variables to apply for an activation, which includes
/// the variables to remove from a previous activation, and the variables that
/// track what is being applied, so that it can all be reversed later.
///
/// Shell aliases are not included, as they are not environment variables, and
/// can only be applied by shells that evaluate our syntax.
fn create_activation_env(
    workflow: &ExecWorkflow,
    shell_type: &ShellType,
) -> miette::Result<IndexMap<String, Option<String>>> {
    let mut env = IndexMap::default();

    // Remove previously set variables
    if let Ok(env_to_remove) = env::var(ACTIVATED_ENV_KEY) {
        for key in env_to_remove.split(',') {
            if !key.is_empty() && !workflow.env.contains_key(key) {
                env.insert(key.to_owned(), None);
            }
        }
    }

    // Set/remove new variables
    let mut env_being_set = vec![];

    for (key, value) in &workflow.env {
        if value.is_some() {
            env_being_set.push(key.as_str());
        }

        env.insert(key.to_owned(), value.to_owned());
    }

    // Track what we've applied, so that it can be reversed
    track_activation(
        &mut env,
        ACTIVATED_ENV_KEY,
        (!env_being_set.is_empty()).then(|| env_being_set.join(",")),
    );

    track_activation(
        &mut env,
        ACTIVATED_PATH_KEY,
        workflow
            .join_activated_paths_for_shell(shell_type)?
            .map(|path| path.to_string_lossy().to_string()),
    );

    Ok(env)
}

/// Record what an activation applied, or stop tracking it when there's nothing
/// left to apply. Variables that were never tracked are left alone, so that we
/// don't emit noise (or errors in strict shells) for a fresh session.
fn track_activation(env: &mut IndexMap<String, Option<String>>, key: &str, value: Option<String>) {
    if value.is_some() || env::var(key).is_ok() {
        env.insert(key.to_owned(), value);
    }
}

fn stringify_paths(paths: Vec<PathBuf>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

fn print_activation_exports(
    session: &ProtoSession,
    shell_type: &ShellType,
    workflow: ExecWorkflow,
) -> miette::Result<()> {
    let shell = shell_type.build();
    let aliases = &session.load_config()?.shell.aliases;
    let mut output = vec![];

    // Set/remove variables
    for (key, value) in create_activation_env(&workflow, shell_type)? {
        output.push(shell.format_env(&key, value.as_deref()));
    }

    // Remove previously set aliases
    if let Ok(alias_to_remove) = env::var(ACTIVATED_ALIASES_KEY) {
        for key in alias_to_remove.split(',') {
            if !key.is_empty() && !aliases.contains_key(key) {
                output.push(shell.format_alias_unset(key));
            }
        }
    }

    // Set/remove new aliases
    if !aliases.is_empty() {
        for (alias, command) in aliases {
            output.push(shell.format_alias_set(alias, command));
        }
    }

    let mut alias_tracking = IndexMap::default();

    track_activation(
        &mut alias_tracking,
        ACTIVATED_ALIASES_KEY,
        (!aliases.is_empty()).then(|| {
            aliases
                .keys()
                .map(|key| key.as_str())
                .collect::<Vec<_>>()
                .join(",")
        }),
    );

    for (key, value) in alias_tracking {
        output.push(shell.format_env(&key, value.as_deref()));
    }

    // Set new `PATH`
    if !workflow.paths.is_empty() {
        let paths =
            stringify_paths(workflow.reset_paths_for_shell(&session.env.store.dir, shell_type));

        if !paths.is_empty() && !is_test() {
            output.push(shell.format_path_set(&paths));
        }
    }

    session.console.out.write_line(output.join("\n"))?;

    Ok(())
}
