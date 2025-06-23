use crate::locator_error::PluginLocatorError;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

/// A file system locator.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FileLocator {
    /// Path explicitly configured by a user (with file://).
    pub file: String,

    /// The file (above) resolved to an absolute path.
    /// This must be done manually on the host side.
    pub path: Option<PathBuf>,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileLocator {
    pub fn get_unresolved_path(&self) -> PathBuf {
        PathBuf::from(self.file.strip_prefix("file://").unwrap_or(&self.file))
    }

    pub fn get_resolved_path(&self) -> PathBuf {
        let mut path = self
            .path
            .clone()
            .unwrap_or_else(|| self.get_unresolved_path());

        if !path.is_absolute() {
            path = std::env::current_dir()
                .expect("Could not determine working directory!")
                .join(path);
        }

        path
    }
}

/// A GitHub release locator.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GitHubLocator {
    /// Owner/org and repository name: `owner/repo`.
    pub repo_slug: String,

    /// Explicit release tag to use. Defaults to `latest`.
    pub tag: Option<String>,

    /// Project name to match tags against. Primarily used in monorepos.
    pub project_name: Option<String>,
}

/// A HTTPS URL locator.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UrlLocator {
    /// URL explicitly configured by a user (with https://).
    pub url: String,
}

/// A OCI Registry locator.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OciLocator {
    /// The image name, e.g., `plugins/python`.
    pub image: String,
}

/// Strategies and protocols for locating plugins.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum PluginLocator {
    /// file:///abs/path/to/file.wasm
    /// file://../rel/path/to/file.wasm
    File(Box<FileLocator>),

    /// github://owner/repo
    /// github://owner/repo@tag
    /// github://owner/repo/project
    GitHub(Box<GitHubLocator>),

    /// https://url/to/file.wasm
    Url(Box<UrlLocator>),

    /// oci://plugins/python
    /// oci://plugins/python:tag
    Oci(Box<OciLocator>),
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
            PluginLocator::File(file) => {
                if file.file.starts_with("file://") {
                    write!(f, "{}", file.file)
                } else {
                    write!(f, "file://{}", file.file)
                }
            }
            PluginLocator::Url(url) => write!(f, "{}", url.url),
            PluginLocator::GitHub(github) => write!(
                f,
                "github://{}{}{}",
                github.repo_slug,
                github
                    .project_name
                    .as_deref()
                    .map(|n| format!("/{n}"))
                    .unwrap_or_default(),
                github
                    .tag
                    .as_deref()
                    .map(|t| format!("@{t}"))
                    .unwrap_or_default()
            ),
            PluginLocator::Oci(oci) => write!(f, "{}", oci.image),
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
        } else if value.starts_with("oci:") && !value.contains("//") {
            return Self::try_from(format!("oci://{}", &value[6..]));
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
            "file" => Ok(PluginLocator::File(Box::new(FileLocator {
                file: value,
                path: None,
            }))),
            "github" => {
                if !location.contains('/') {
                    return Err(PluginLocatorError::MissingGitHubOrg);
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

                github.project_name = prefix;
                github.repo_slug = format!("{org}/{repo}");

                Ok(PluginLocator::GitHub(Box::new(github)))
            }
            "http" => Err(PluginLocatorError::SecureUrlsOnly),
            "https" => Ok(PluginLocator::Url(Box::new(UrlLocator { url: value }))),
            "oci" => Ok(PluginLocator::Oci(Box::new(OciLocator { image: value }))),
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
