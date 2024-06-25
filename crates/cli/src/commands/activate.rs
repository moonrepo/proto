use crate::session::ProtoSession;
use clap::Args;
use starbase::AppResult;
use starbase_shell::ShellType;

#[derive(Args, Clone, Debug)]
pub struct ActivateArgs {
    #[arg(help = "Shell to activate for")]
    shell: Option<ShellType>,
}

#[tracing::instrument(skip_all)]
pub async fn activate(session: ProtoSession, args: ActivateArgs) -> AppResult {
    // Detect the shell that we need to activate for
    let shell = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    // Load all the tools so that we can update PATH
    let tools = session.load_tools().await?;

    Ok(())
}
