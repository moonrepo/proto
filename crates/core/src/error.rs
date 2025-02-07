use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoErrorOld {
    #[error("{0}")]
    Message(String),

    #[diagnostic(code(proto::tool::build_failed))]
    #[error("Failed to build {tool} from {}: {status}", .url.style(Style::Url))]
    BuildFailed {
        tool: String,
        url: String,
        status: String,
    },

    #[diagnostic(code(proto::offline))]
    #[error("Internet connection required, unable to download, install, or run tools.")]
    InternetConnectionRequired,

    #[diagnostic(code(proto::tool::required))]
    #[error(
        "This project requires {tool} {}, but this version has not been installed. Install it with {}, or enable the {} setting to automatically install missing versions!",
        .version.style(Style::Hash),
        .command.style(Style::Shell),
        "auto-install".style(Style::Property),
    )]
    MissingToolForRun {
        tool: String,
        version: String,
        command: String,
    },

    #[diagnostic(code(proto::tool::required))]
    #[error(
        "This project requires {tool} {} (detected from {}), but this version has not been installed. Install it with {}, or enable the {} setting to automatically install missing versions!",
        .version.style(Style::Hash),
        .path.style(Style::Path),
        .command.style(Style::Shell),
        "auto-install".style(Style::Property),
    )]
    MissingToolForRunWithSource {
        tool: String,
        version: String,
        command: String,
        path: PathBuf,
    },

    #[diagnostic(code(proto::http))]
    #[error("Failed to request {}.", .url.style(Style::Url))]
    Http {
        url: String,
        #[source]
        error: Box<reqwest::Error>,
    },
}
