use serde::Serialize;
use warpgate::test_utils::ConfigBuilder;

pub trait ProtoConfigBuilder {
    fn schema_config(&mut self, schema: serde_json::Value) -> &mut Self;
    fn backend_config(&mut self, config: impl Serialize) -> &mut Self;
    fn backend_id(&mut self, id: String) -> &mut Self;
    fn tool_config(&mut self, config: impl Serialize) -> &mut Self;
    fn tool_id(&mut self, id: String) -> &mut Self;
}

impl ProtoConfigBuilder for ConfigBuilder {
    fn schema_config(&mut self, schema: serde_json::Value) -> &mut Self {
        self.insert("proto_schema", schema)
    }

    fn backend_config(&mut self, config: impl Serialize) -> &mut Self {
        self.insert("proto_backend_config", config)
    }

    fn backend_id(&mut self, id: String) -> &mut Self {
        self.insert("proto_backend_id", id)
    }

    fn tool_config(&mut self, config: impl Serialize) -> &mut Self {
        self.insert("proto_tool_config", config)
    }

    fn tool_id(&mut self, id: String) -> &mut Self {
        self.plugin_id(id.clone());
        self.insert("proto_tool_id", id)
    }
}
