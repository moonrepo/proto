use rustc_hash::FxHashMap;
use serde_json::Value;
use warpgate_api::{AnyResult, anyhow, api_struct, select_json_path};

// Note: We don't use `api_struct!` for named enums because they are not supported by schematic!

/// Input passed to the `load_tool_versions` host function.
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LoadToolVersionsInput {
    /// Load versions from Git tags.
    Git {
        /// The URL of the Git remote.
        url: String,

        /// A regex pattern to match Git tags.
        #[serde(default)]
        tag_pattern: Option<String>,
    },

    /// Load versions from a JSON index.
    Index {
        /// The URL of the JSON index.
        url: String,

        /// A JSON path to the version(s) in the index.
        #[serde(default, alias = "version_path")]
        version_key: Option<String>,

        /// HTTP headers to send with the request. Only sent to this URL,
        /// not to an index URL configured by the user.
        #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
        headers: FxHashMap<String, String>,

        /// Return the response body in the output, for data other than
        /// versions, like aliases. Only returned for this URL, not for a
        /// source configured by the user.
        #[serde(default)]
        include_body: bool,
    },
}

impl LoadToolVersionsInput {
    /// Create a `LoadToolVersionsInput` for loading versions from a Git remote.
    pub fn git(url: impl AsRef<str>, tag_pattern: Option<String>) -> Self {
        Self::Git {
            url: url.as_ref().to_owned(),
            tag_pattern,
        }
    }

    /// Create a `LoadToolVersionsInput` for loading versions from a JSON index.
    pub fn index(url: impl AsRef<str>, version_key: Option<String>) -> Self {
        Self::Index {
            url: url.as_ref().to_owned(),
            version_key,
            headers: FxHashMap::default(),
            include_body: false,
        }
    }
}

api_struct!(
    /// Output returned from the `load_tool_versions` host function.
    pub struct LoadToolVersionsOutput {
        /// Available versions loaded from a Git remote or index.
        pub versions: Vec<String>,

        /// The index response body, when requested with `include_body`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub body: Option<String>,
    }
);

/// Select the versions at a JSON path of an index. A single array
/// is selected as its items, and every version must be a string.
pub fn select_index_versions(index: &Value, path: &str) -> AnyResult<Vec<String>> {
    let values = select_json_path(index, path)?;
    let values: Vec<&Value> = match values.as_slice() {
        [value] => match value.as_array() {
            Some(array) => array.iter().collect(),
            None => vec![value],
        },
        _ => values.iter().map(|value| value.as_ref()).collect(),
    };

    values
        .into_iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                anyhow!("Tool versions at path {path:?} must be a string or contain only strings")
            })
        })
        .collect()
}
