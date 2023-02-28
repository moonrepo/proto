use clap::ValueEnum;
use std::str::FromStr;

pub use proto_bun as bun;
pub use proto_core::*;
pub use proto_deno as deno;
pub use proto_go as go;
pub use proto_node as node;

#[derive(Clone, Debug, Eq, Hash, PartialEq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum ToolType {
    // Bun
    Bun,
    // Deno
    Deno,
    // Go
    Go,
    // Node.js
    Node,
    Npm,
    Pnpm,
    Yarn,
}

impl FromStr for ToolType {
    type Err = ProtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_ref() {
            // Bun
            "bun" => Ok(Self::Bun),
            // Deno
            "deno" => Ok(Self::Deno),
            // Go
            "go" => Ok(Self::Go),
            // Node.js
            "node" => Ok(Self::Node),
            "npm" => Ok(Self::Npm),
            "pnpm" => Ok(Self::Pnpm),
            "yarn" | "yarnpkg" => Ok(Self::Yarn),
            _ => Err(ProtoError::UnsupportedTool(value.to_owned())),
        }
    }
}

pub fn create_tool(tool: &ToolType) -> Result<Box<dyn Tool<'static>>, ProtoError> {
    let proto = Proto::new()?;

    Ok(match tool {
        // Bun
        ToolType::Bun => Box::new(bun::BunLanguage::new(&proto)),
        // Deno
        ToolType::Deno => Box::new(deno::DenoLanguage::new(&proto)),
        // Go
        ToolType::Go => Box::new(go::GoLanguage::new(&proto)),
        // Node.js
        ToolType::Node => Box::new(node::NodeLanguage::new(&proto)),
        ToolType::Npm => Box::new(node::NodeDependencyManager::new(
            &proto,
            node::NodeDependencyManagerType::Npm,
        )),
        ToolType::Pnpm => Box::new(node::NodeDependencyManager::new(
            &proto,
            node::NodeDependencyManagerType::Pnpm,
        )),
        ToolType::Yarn => Box::new(node::NodeDependencyManager::new(
            &proto,
            node::NodeDependencyManagerType::Yarn,
        )),
    })
}
