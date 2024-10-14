use serde::Serialize;
use warpgate::test_utils::ConfigBuilder;

pub trait ProtoConfigBuilder {
    fn schema_config(&mut self, schema: serde_json::Value) -> &mut Self;
    fn tool_config(&mut self, config: impl Serialize) -> &mut Self;
}

impl ProtoConfigBuilder for ConfigBuilder {
    fn schema_config(&mut self, schema: serde_json::Value) -> &mut Self {
        self.insert("proto_schema", schema)
    }

    fn tool_config(&mut self, config: impl Serialize) -> &mut Self {
        self.insert("proto_tool_config", config)
    }
}
