use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

/// A GitHub release locator.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GitHubLocator {
    /// Organization and repository slug: `owner/repo`.
    pub repo_slug: String,

    /// Explicit release tag to use. Defaults to `latest`.
    pub tag: Option<String>,

    /// Tag prefix to find releases against. Primarily used in monorepos.
    pub tag_prefix: Option<String>,
}

impl GitHubLocator {
    pub fn extract_prefix_from_slug(slug: &str) -> &str {
        slug.split('/').next().expect("Expected an owner scope!")
    }

    pub fn extract_suffix_from_slug(slug: &str) -> &str {
        slug.split('/').nth(1).expect("Expected a repository name!")
    }
}

/// Errors during plugin locator parsing.
#[derive(thiserror::Error, Debug)]
pub enum PluginLocatorError {
    #[error("GitHub release locator requires a repository with organization scope (org/repo).")]
    GitHubMissingOrg,

    #[error("Missing plugin location (after protocol).")]
    MissingLocation,

    #[error("Missing plugin protocol.")]
    MissingProtocol,

    #[error("Only https URLs are supported for plugins.")]
    SecureUrlsOnly,

    #[error("Unknown plugin protocol `{0}`.")]
    UnknownProtocol(String),
}

/// Strategies and protocols for locating plugins.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum PluginLocator {
    /// file:///abs/path/to/file.wasm
    /// file://../rel/path/to/file.wasm
    File {
        /// Configured path (without file://).
        file: String,
        /// Resolved absolute path.
        path: Option<PathBuf>,
    },

    /// github://owner/repo
    /// github://owner/repo@tag
    GitHub(Box<GitHubLocator>),

    /// https://url/to/file.wasm
    Url {
        /// Configured URL (with https://).
        url: String,
    },
}

impl PluginLocator {
    pub fn create_wasm_file_prefix(name: &str) -> String {
        let mut name = name.to_lowercase().replace('-', "_");

        if !name.ends_with("_plugin") {
            name.push_str("_plugin");
        }

        name
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for PluginLocator {
    fn schema_name() -> Option<String> {
        Some("PluginLocator".into())
    }

    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.set_description("Strategies and protocols for locating plugins.");
        schema.string_default()
    }
}

impl Display for PluginLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginLocator::File { file, .. } => write!(f, "file://{}", file),
            PluginLocator::Url { url } => write!(f, "{}", url),
            PluginLocator::GitHub(github) => write!(
                f,
                "github://{}{}",
                github.repo_slug,
                github
                    .tag
                    .as_deref()
                    .map(|t| format!("@{t}"))
                    .unwrap_or_default()
            ),
        }
    }
}

impl FromStr for PluginLocator {
    type Err = PluginLocatorError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        PluginLocator::try_from(value.to_owned())
    }
}

impl TryFrom<String> for PluginLocator {
    type Error = PluginLocatorError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // Legacy support
        if let Some(source) = value.strip_prefix("source:") {
            if source.starts_with("http") {
                return Self::try_from(source.to_owned());
            } else {
                return Self::try_from(format!("file://{source}"));
            }
        } else if value.starts_with("github:") && !value.contains("//") {
            return Self::try_from(format!("github://{}", &value[7..]));
        }

        if !value.contains("://") {
            return Err(PluginLocatorError::MissingProtocol);
        }

        let mut parts = value.splitn(2, "://");

        let Some(protocol) = parts.next() else {
            return Err(PluginLocatorError::MissingProtocol);
        };

        let Some(location) = parts.next() else {
            return Err(PluginLocatorError::MissingLocation);
        };

        if location.is_empty() {
            return Err(PluginLocatorError::MissingLocation);
        }

        match protocol {
            "file" => Ok(PluginLocator::File {
                file: location.to_owned(),
                path: None,
            }),
            "github" => {
                if !location.contains('/') {
                    return Err(PluginLocatorError::GitHubMissingOrg);
                }

                let mut github = GitHubLocator::default();
                let mut query = location;

                if let Some(index) = query.find('@') {
                    github.tag = Some(query[index + 1..].into());
                    query = &query[0..index];
                }

                let mut parts = query.split('/');
                let org = parts.next().unwrap_or_default().to_owned();
                let repo = parts.next().unwrap_or_default().to_owned();
                let prefix = parts.next().map(|f| f.to_owned());

                github.tag_prefix = prefix;
                github.repo_slug = format!("{org}/{repo}");

                Ok(PluginLocator::GitHub(Box::new(github)))
            }
            "http" => Err(PluginLocatorError::SecureUrlsOnly),
            "https" => Ok(PluginLocator::Url { url: value }),
            unknown => Err(PluginLocatorError::UnknownProtocol(unknown.to_owned())),
        }
    }
}

impl From<PluginLocator> for String {
    fn from(locator: PluginLocator) -> Self {
        locator.to_string()
    }
}

impl AsRef<PluginLocator> for PluginLocator {
    fn as_ref(&self) -> &Self {
        self
    }
}
