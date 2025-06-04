use thiserror::Error;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoEnvError {
    #[diagnostic(code(proto::env::home_dir))]
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[diagnostic(code(proto::env::working_dir))]
    #[error("Unable to determine current working directory!")]
    MissingWorkingDir,
}

unsafe impl Send for ProtoEnvError {}
unsafe impl Sync for ProtoEnvError {}
