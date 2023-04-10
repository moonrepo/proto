use crate::app::App;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use proto_core::ProtoError;
use starbase::SystemResult;

pub async fn completions(shell: Option<Shell>) -> SystemResult {
    let Some(shell) = shell.or_else(Shell::from_env) else {
      return Err(ProtoError::UnsupportedShell)?;
    };

    let mut app = App::command();
    let mut stdio = std::io::stdout();

    generate(shell, &mut app, "proto", &mut stdio);

    Ok(())
}
