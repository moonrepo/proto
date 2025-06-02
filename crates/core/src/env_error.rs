use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoEnvError {
    #[cfg_attr(feature = "miette", diagnostic(code(proto::env::home_dir)))]
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[cfg_attr(feature = "miette", diagnostic(code(proto::env::working_dir)))]
    #[error("Unable to determine current working directory!")]
    MissingWorkingDir,

    #[cfg_attr(feature = "miette", diagnostic(code(proto::offline)))]
    #[error("Internet connection required, unable to download, install, or run tools.")]
    RequiredInternetConnection,
}
