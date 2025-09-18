use crate::error::ProtoCliError;
use crate::session::{LoadToolOptions, ProtoSession};
use crate::workflows::{ExecWorkflow, ExecWorkflowParams};
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{ToolContext, ToolSpec};
use proto_shim::exec_command_and_replace;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase::AppResult;
use starbase_shell::ShellType;

#[derive(Args, Clone, Debug)]
pub struct ExecArgs {
    #[arg(help = "Tools to initialize")]
    tools: Vec<String>,

    #[arg(long, help = "Inherit tools to initialize from .prototools configs")]
    tools_from_config: bool,

    #[arg(long, help = "Execute the command as-is without quoting or escaping")]
    raw: bool,

    #[arg(long, help = "Shell to execute the command with")]
    shell: Option<ShellType>,

    // Passthrough args (after --)
    #[arg(last = true, help = "The command to execute after initializing tools")]
    command: Vec<String>,
}

#[tracing::instrument(skip_all)]
pub async fn exec(session: ProtoSession, mut args: ExecArgs) -> AppResult {
    if args.command.is_empty() {
        return Err(ProtoCliError::ExecMissingCommand.into());
    }

    let config = session.load_config()?;
    let mut specs = FxHashMap::default();

    for value in &args.tools {
        // We need to check if the string contains `@<version>` to properly
        // parse the context and spec, but this becomes complicated with
        // npm packages that have an `@` scope. We need to support all these:
        //  - npm:@scope/org
        //  - npm:@scope/org@version
        //  - tool
        //  - tool@version
        let has_version = value.chars().filter(|c| *c == '@').count() >= 1 && !value.contains(":@");

        if has_version && let Some(index) = value.rfind('@') {
            specs.insert(
                ToolContext::parse(&value[0..index])?,
                Some(ToolSpec::parse(&value[index + 1..])?),
            );
        } else {
            specs.insert(ToolContext::parse(value)?, None);
        }
    }

    if args.tools_from_config {
        for (context, spec) in &config.versions {
            if !specs.contains_key(context) {
                specs.insert(context.to_owned(), Some(spec.to_owned()));
            }
        }
    }

    // Load tools
    let tools = session
        .load_tools_with_options(LoadToolOptions {
            tools: FxHashSet::from_iter(specs.keys().cloned()),
            ..Default::default()
        })
        .await?;

    // Prepare environment
    let mut workflow = ExecWorkflow::new(tools, config);

    workflow
        .prepare_environment(
            specs
                .into_iter()
                .filter_map(|(ctx, spec)| spec.map(|s| (ctx, s)))
                .collect(),
            ExecWorkflowParams {
                activate_environment: true,
                detect_version: true,
                pre_run_hook: true,
                version_env_vars: true,
                ..Default::default()
            },
        )
        .await?;

    // Create and run command
    let command = match args.shell {
        None => workflow.create_command(args.command.remove(0), args.command)?,
        Some(shell) => workflow.create_command_with_shell(
            shell.build(),
            args.command.remove(0),
            args.command,
            args.raw,
        )?,
    };

    // Must be the last line!
    exec_command_and_replace(command)
        .into_diagnostic()
        .map(|_| None)
}
