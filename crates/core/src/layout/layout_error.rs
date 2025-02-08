use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoLayoutError {
    #[diagnostic(code(proto::store::shim::create_failed))]
    #[error("Failed to create shim {}.", .path.style(Style::Path))]
    FailedCreateShim {
        path: PathBuf,
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(proto::store::shim::missing_binary))]
    #[error(
        "Unable to create shims as the {} binary cannot be found.\nLooked in the {} environment variable and {} directory.",
        "proto-shim".style(Style::Id),
        "PROTO_HOME".style(Style::Property),
        .bin_dir.style(Style::Path),
    )]
    MissingShimBinary { bin_dir: PathBuf },
}
