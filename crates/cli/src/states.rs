use proto_core::{ToolsConfig as InnerToolsConfig, UserConfig as InnerUserConfig};
use starbase::State;

#[derive(State)]
pub struct ToolsConfig(pub Option<InnerToolsConfig>);

#[derive(State)]
pub struct UserConfig(pub InnerUserConfig);
