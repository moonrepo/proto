use crate::config::PROTO_CONFIG_NAME;
use crate::tool_spec::Backend;
use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;
use warpgate::Id;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoToolError {
    #[diagnostic(code(proto::tool::minimum_version_requirement))]
    #[error(
        "Unable to use the {tool} plugin with identifier {}, as it requires a minimum proto version of {}, but found {} instead.",
        .id.to_string().style(Style::Id),
        .expected.style(Style::Hash),
        .actual.style(Style::Hash)
    )]
    InvalidMinimumVersion {
        tool: String,
        id: Id,
        expected: String,
        actual: String,
    },

    #[diagnostic(code(proto::tool::invalid_spec))]
    #[error("Invalid version or requirement in tool specification {}.", .spec.style(Style::Hash))]
    InvalidVersionSpec {
        spec: String,
        #[source]
        error: Box<version_spec::SpecError>,
    },

    #[diagnostic(code(proto::tool::invalid_inventory_dir))]
    #[error("{tool} inventory directory has been overridden with {} but it's not an absolute path. Only absolute paths are supported.", .dir.style(Style::Path))]
    RequiredAbsoluteInventoryDir { tool: String, dir: PathBuf },

    #[diagnostic(code(proto::tool::unknown_backend))]
    #[error(
        "Unknown backend in tool specification {}. Only {} are supported.",
        .spec.style(Style::Hash),
        .backends.iter().map(|be| be.to_string().style(Style::Id)).collect::<Vec<_>>().join(", ")
    )]
    UnknownBackend {
        backends: Vec<Backend>,
        spec: String,
    },

    #[diagnostic(code(proto::tool::unknown_id))]
    #[error(
        "Unable to proceed, {} is not a built-in plugin and has not been configured with {} in a {} file.\n\nLearn more about plugins: {}\nSearch community plugins: {}",
        .id.to_string().style(Style::Id),
        "[plugins]".style(Style::Property),
        PROTO_CONFIG_NAME.style(Style::File),
        "https://moonrepo.dev/docs/proto/plugins".style(Style::Url),
        format!("proto plugin search {}", .id).style(Style::Shell),
    )]
    UnknownTool { id: Id },
}
