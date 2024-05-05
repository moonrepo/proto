use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

/// A GitHub release locator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitHubLocator {
    /// Name of asset without extension.
    /// Defaults to `<repo>_plugin`.
    pub file_prefix: String,

    /// Organization and repository slug: `owner/repo`.
    pub repo_slug: String,

    /// Release tag to use. Defaults to `latest`.
    pub tag: Option<String>,
}

/// Errors during plugin locator parsing.
#[derive(thiserror::Error, Debug)]
pub enum PluginLocatorError {
    #[error("GitHub release locator requires a repository with organization scope (org/repo).")]
    GitHubMissingOrg,

    #[error("Missing plugin location (after :).")]
    MissingLocation,

    #[error("Missing plugin scope or location.")]
    MissingScope,

    #[error("Only https URLs are supported for source plugins.")]
    SecureUrlsOnly,

    #[error("Unknown plugin scope `{0}`.")]
    UnknownScope(String),
}

/// Strategies for locating plugins.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum PluginLocator {
    /// source:path/to/file.wasm
    SourceFile { file: String, path: PathBuf },

    /// source:https://url/to/file.wasm
    SourceUrl { url: String },

    /// github:owner/repo
    /// github:owner/repo@tag
    GitHub(Box<GitHubLocator>),
}

impl PluginLocator {
    pub fn extract_prefix_from_slug(slug: &str) -> &str {
        slug.split('/').next().expect("Expected an owner scope!")
    }

    pub fn extract_suffix_from_slug(slug: &str) -> &str {
        slug.split('/')
            .nth(1)
            .expect("Expected a package or repository name!")
    }

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
    fn generate_schema() -> schematic::SchemaType {
        schematic::SchemaType::string()
    }
}

impl Display for PluginLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginLocator::SourceFile { file, .. } => write!(f, "source:{}", file),
            PluginLocator::SourceUrl { url } => write!(f, "source:{}", url),
            PluginLocator::GitHub(github) => write!(
                f,
                "github:{}{}",
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
        let mut parts = value.splitn(2, ':');

        let Some(scope) = parts.next() else {
            return Err(PluginLocatorError::MissingScope);
        };

        let Some(location) = parts.next() else {
            return Err(PluginLocatorError::MissingScope);
        };

        if location.is_empty() {
            return Err(PluginLocatorError::MissingLocation);
        }

        match scope {
            "source" => {
                if location.starts_with("http:") {
                    Err(PluginLocatorError::SecureUrlsOnly)
                } else if location.starts_with("https:") {
                    Ok(PluginLocator::SourceUrl {
                        url: location.to_owned(),
                    })
                } else {
                    Ok(PluginLocator::SourceFile {
                        file: location.to_owned(),
                        path: PathBuf::from(location),
                    })
                }
            }
            "github" => {
                if !location.contains('/') {
                    return Err(PluginLocatorError::GitHubMissingOrg);
                }

                let mut parts = location.splitn(2, '@');
                let repo_slug = parts.next().unwrap().to_owned();
                let tag = parts.next().map(|t| t.to_owned());

                Ok(PluginLocator::GitHub(Box::new(GitHubLocator {
                    file_prefix: PluginLocator::create_wasm_file_prefix(
                        PluginLocator::extract_suffix_from_slug(&repo_slug),
                    ),
                    repo_slug,
                    tag,
                })))
            }
            unknown => Err(PluginLocatorError::UnknownScope(unknown.to_owned())),
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
