use warpgate_api::api_struct;

// Note: We don't use `api_struct!` for named enums because they are not supported by schematic!

/// Input passed to the `load_tool_versions` host function.
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LoadToolVersionsInput {
    /// Load versions from Git tags.
    Git {
        url: String,
        #[serde(default)]
        tag_pattern: Option<String>,
    },

    /// Load versions from a JSON index.
    Index {
        url: String,
        #[serde(default, alias = "version_path")]
        version_key: Option<String>,
    },
}

api_struct!(
    /// Output returned from the `load_tool_versions` host function.
    pub struct LoadToolVersionsOutput {
        /// Available versions loaded from a Git remote or index.
        pub versions: Vec<String>,
    }
);
