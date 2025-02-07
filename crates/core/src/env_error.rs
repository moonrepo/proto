use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoEnvError {
    #[diagnostic(code(proto::env::home_dir))]
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[diagnostic(code(proto::env::working_dir))]
    #[error("Unable to determine current working directory!")]
    MissingWorkingDir,

    #[diagnostic(code(proto::offline))]
    #[error("Internet connection required, unable to download, install, or run tools.")]
    RequiredInternetConnection,
}
