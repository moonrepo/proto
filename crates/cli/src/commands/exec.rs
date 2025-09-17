use crate::session::{LoadToolOptions, ProtoSession};
use crate::workflows::{ExecWorkflow, ExecWorkflowParams};
use clap::Args;
use proto_core::{ToolContext, ToolSpec};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase::AppResult;
use starbase_shell::ShellType;

#[derive(Args, Clone, Debug)]
pub struct ExecArgs {
    #[arg(required = true, help = "Tools to initialize")]
    tools: Vec<String>,

    #[arg(help = "Shell to execute with")]
    shell: Option<ShellType>,

    // Passthrough args (after --)
    #[arg(last = true, help = "The command to execute after initializing tools")]
    command: Vec<String>,
}

#[tracing::instrument(skip_all)]
pub async fn exec(session: ProtoSession, args: ExecArgs) -> AppResult {
    // Detect the shell that we need to activate for
    let shell_type = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    // Extract contexts and specs
    let mut specs = FxHashMap::default();

    for value in &args.tools {
        if let Some(index) = value.rfind('@') {
            specs.insert(
                ToolContext::parse(&value[0..index])?,
                Some(ToolSpec::parse(&value[index + 1..])?),
            );
        } else {
            specs.insert(ToolContext::parse(value)?, None);
        }
    }

    // Load config and tools
    let config = session.load_config()?;
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
    let shell_command = shell_type.build().get_exec_command();

    Ok(None)
}
