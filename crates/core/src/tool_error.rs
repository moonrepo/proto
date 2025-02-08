use crate::config::PROTO_CONFIG_NAME;
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

    #[diagnostic(code(proto::tool::invalid_inventory_dir))]
    #[error("{tool} inventory directory has been overridden with {} but it's not an absolute path. Only absolute paths are supported.", .dir.style(Style::Path))]
    RequiredAbsoluteInventoryDir { tool: String, dir: PathBuf },

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
