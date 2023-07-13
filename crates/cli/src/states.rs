use proto_core::{ToolsConfig as InnerToolsConfig, UserConfig as InnerUserConfig};
use starbase::State;

#[derive(State)]
pub struct MergedToolsConfig(pub InnerToolsConfig);

#[derive(State)]
pub struct UserConfig(pub InnerUserConfig);

#[derive(Debug, State)]
pub struct PluginList(pub Vec<String>);
