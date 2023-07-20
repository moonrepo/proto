use crate::helpers::{create_wasm_file_stem, extract_suffix_from_slug};
use crate::WarpgateError;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitHubLocator {
    pub file_stem: String, // Without extension
    pub repo_slug: String,
    pub tag: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WapmLocator {
    pub file_stem: String, // Without extension
    pub package_name: String,
    pub version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum PluginLocator {
    // source:path/to/file.wasm
    SourceFile { file: String, path: PathBuf },

    // source:https://url/to/file.wasm
    SourceUrl { url: String },

    // github:owner/repo@tag
    GitHub(GitHubLocator),

    // wapm:package/name@version
    Wapm(WapmLocator),
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
            PluginLocator::Wapm(wapm) => write!(
                f,
                "wapm:{}{}",
                wapm.package_name,
                wapm.version
                    .as_deref()
                    .map(|v| format!("@{v}"))
                    .unwrap_or_default()
            ),
        }
    }
}

impl TryFrom<String> for PluginLocator {
    type Error = WarpgateError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut parts = value.splitn(2, ':');

        let Some(scope) = parts.next() else {
            return Err(WarpgateError::Serde("Missing plugin scope or location.".into()));
        };

        let Some(location) = parts.next() else {
            return Err(WarpgateError::Serde("Missing plugin scope or location.".into()));
        };

        if location.is_empty() {
            return Err(WarpgateError::Serde(
                "Missing plugin location (after :).".into(),
            ));
        }

        match scope {
            "source" => {
                if location.starts_with("http:") {
                    Err(WarpgateError::Serde(
                        "Only https:// URLs are supported for source plugins.".into(),
                    ))
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
                    return Err(WarpgateError::Serde(
                        "GitHub release locator requires a repository with organization scope (org/repo)."
                            .into(),
                    ));
                }

                let mut parts = location.splitn(2, '@');
                let repo_slug = parts.next().unwrap().to_owned();
                let tag = parts.next().map(|t| t.to_owned());

                Ok(PluginLocator::GitHub(GitHubLocator {
                    file_stem: create_wasm_file_stem(extract_suffix_from_slug(&repo_slug)),
                    repo_slug,
                    tag,
                }))
            }
            "wapm" => {
                if !location.contains('/') {
                    return Err(WarpgateError::Serde(
                        "wapm.io locator requires a package with owner scope (owner/package)."
                            .into(),
                    ));
                }

                let mut parts = location.splitn(2, '@');
                let package_name = parts.next().unwrap().to_owned();
                let version = parts.next().map(|t| t.to_owned());

                Ok(PluginLocator::Wapm(WapmLocator {
                    file_stem: create_wasm_file_stem(extract_suffix_from_slug(&package_name)),
                    package_name,
                    version,
                }))
            }
            unknown => Err(WarpgateError::Serde(format!(
                "Unknown plugin scope `{unknown}`."
            ))),
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
