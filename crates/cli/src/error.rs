use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoCliError {
    #[diagnostic(code(proto::cli::no_self_upgrade))]
    #[error(
        "Self upgrading {} is not supported in proto, as it conflicts with proto's managed inventory.\nUse {} instead to upgrade to the latest version.",
        .name,
        .command.style(Style::Shell)
    )]
    NoSelfUpgrade { command: String, name: String },
}
