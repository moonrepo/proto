use crate::app::App;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use proto_core::ProtoError;

pub async fn completions(shell: Option<Shell>) -> Result<(), ProtoError> {
    let Some(shell) = shell.or_else(Shell::from_env) else {
      return Err(ProtoError::UnsupportedShell);
    };

    let mut app = App::command();
    let mut stdio = std::io::stdout();

    generate(shell, &mut app, "proto", &mut stdio);

    Ok(())
}
