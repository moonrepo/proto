use crate::errors::ProtoError;
use crate::helpers::{get_temp_dir, is_alias_name, is_offline, remove_v_prefix};
use human_sort::compare;
use lenient_semver::Version;
use rustc_hash::FxHashMap;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use starbase_styles::color;
use starbase_utils::{fs, json, json::JsonError};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};
use tokio::process::Command;
use tracing::{debug, trace};

#[derive(Debug)]
pub struct VersionManifestEntry {
    pub alias: Option<String>,
    pub version: String,
}

#[derive(Debug, Default)]
pub struct VersionManifest {
    pub aliases: BTreeMap<String, String>,
    pub versions: BTreeMap<String, VersionManifestEntry>,
}

impl VersionManifest {
    pub fn find_version<V: AsRef<str>>(&self, version: V) -> Result<&String, ProtoError> {
        let mut version = version.as_ref();

        if is_alias_name(version) {
            version = self.get_version_from_alias(version)?;
        }

        let prefixless_version = remove_v_prefix(version);

        // Matching against explicit version
        if let Some(entry) = self.versions.get(&prefixless_version) {
            return Ok(&entry.version);
        }

        // If all 3 parts of a version were provided, find an exact match
        let exact_match = prefixless_version.split('.').collect::<Vec<_>>().len() >= 3;

        // Match against a partial minor/patch range, for example, "10" -> "10.1.2".
        // We also parse versions instead of using starts with, as we need to ensure
        // "10.1" matches "10.1.*" and not "10.10.*"!
        let find_version = parse_version(&prefixless_version)?;
        let mut latest_matching_version = Version::new(0, 0, 0);
        let mut matched = false;

        for entry in self.versions.values().rev() {
            let entry_version = parse_version(&entry.version)?;

            if exact_match && entry_version == find_version {
                return Ok(&entry.version);
            }

            if entry_version.major != find_version.major {
                continue;
            }

            if find_version.minor != 0 && entry_version.minor != find_version.minor {
                continue;
            }

            if find_version.patch != 0 && entry_version.patch != find_version.patch {
                continue;
            }

            // Find the latest (highest) matching version
            if entry_version > latest_matching_version {
                latest_matching_version = entry_version;
                matched = true;
            }
        }

        // Find again using an explicit match
        if matched {
            return self.find_version(latest_matching_version.to_string());
        }

        Err(ProtoError::VersionResolveFailed(
            prefixless_version.to_owned(),
        ))
    }

    pub fn get_version_from_alias(&self, alias: &str) -> Result<&String, ProtoError> {
        let version = self
            .aliases
            .get(alias)
            .ok_or_else(|| ProtoError::VersionUnknownAlias(alias.to_owned()))?;

        if is_alias_name(version) {
            return self.get_version_from_alias(version);
        }

        Ok(version)
    }

    pub fn get_version(&self, version: &str) -> Result<&String, ProtoError> {
        if let Some(entry) = self.versions.get(version) {
            return Ok(&entry.version);
        }

        Err(ProtoError::VersionResolveFailed(version.to_owned()))
    }

    pub fn inherit_aliases(&mut self, aliases: &FxHashMap<String, String>) {
        for (alias, version) in aliases {
            self.aliases.insert(alias.to_owned(), version.to_owned());
        }
    }
}

#[async_trait::async_trait]
pub trait Resolvable<'tool>: Send + Sync {
    /// Return the version to be used as the global default.
    fn get_default_version(&self) -> Option<&str> {
        None
    }

    /// Return the resolved version.
    fn get_resolved_version(&self) -> &str;

    /// Load the upstream version and release manifest.
    async fn load_version_manifest(&self) -> Result<VersionManifest, ProtoError>;

    /// Given an initial version, resolve it to a fully qualifed and semantic version
    /// according to the tool's ecosystem.
    async fn resolve_version(&mut self, initial_version: &str) -> Result<String, ProtoError> {
        if self.get_resolved_version() != "latest" {
            return Ok(self.get_resolved_version().to_owned());
        }

        let initial_version = remove_v_prefix(initial_version).to_lowercase();

        // If offline but we have a fully qualified semantic version,
        // exit early and assume the version is legitimate
        if is_semantic_version(&initial_version) && is_offline() {
            self.set_version(&initial_version);

            return Ok(initial_version);
        }

        debug!("Resolving a semantic version for \"{}\"", initial_version);

        let manifest = self.load_version_manifest().await?;
        let candidate = manifest.find_version(&initial_version)?;

        debug!("Resolved to {}", candidate);

        self.set_version(candidate);

        Ok(candidate.to_owned())
    }

    /// Explicitly set the resolved version.
    fn set_version(&mut self, version: &str);
}

#[tracing::instrument(skip_all)]
pub async fn load_git_tags<U>(url: U) -> Result<Vec<String>, ProtoError>
where
    U: AsRef<str>,
{
    let url = url.as_ref();

    let output = match Command::new("git")
        .args(["ls-remote", "--tags", "--sort", "version:refname", url])
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => {
            return Err(ProtoError::DownloadFailed(
                url.to_string(),
                format!("Could not list versions from git: {e}"),
            ));
        }
    };

    let Ok(raw) = String::from_utf8(output.stdout) else {
        return Err(ProtoError::DownloadFailed(
            url.to_string(),
            "Failed to parse version list.".into(),
        ));
    };

    let mut tags: Vec<String> = vec![];

    for line in raw.split('\n') {
        let parts: Vec<&str> = line.split('\t').collect();

        if parts.len() < 2 {
            continue;
        }

        tags.push(parts[1].strip_prefix("refs/tags/").unwrap().to_owned());

        // let tag: Vec<&str> = parts[1].split('/').collect();

        // if tag.len() < 3 {
        //     continue;
        // }

        // if let Some(last) = tag.last() {
        //     tags.push((**last).to_owned());
        // }
    }

    tags.sort_by(|a, d| compare(a, d));

    Ok(tags)
}

#[tracing::instrument(skip_all)]
pub fn create_version_manifest_from_tags(tags: Vec<String>) -> VersionManifest {
    let mut latest = Version::new(0, 0, 0);
    let mut aliases = BTreeMap::new();
    let mut versions = BTreeMap::new();

    for tag in &tags {
        if let Ok(version) = Version::parse(tag) {
            let entry = VersionManifestEntry {
                alias: None,
                version: version.to_string(),
            };

            if version > latest {
                latest = version.clone();
            }

            versions.insert(entry.version.clone(), entry);
        }
    }

    if let Some(latest_version) = versions.get_mut(&latest.to_string()) {
        latest_version.alias = Some("latest".into());
    }

    aliases.insert("latest".into(), latest.to_string());

    VersionManifest { aliases, versions }
}

#[tracing::instrument(skip_all)]
pub async fn load_versions_manifest<T, U>(url: U) -> Result<T, ProtoError>
where
    T: DeserializeOwned,
    U: AsRef<str>,
{
    let url = url.as_ref();
    let mut sha = Sha256::new();
    sha.update(url);

    let temp_dir = get_temp_dir()?;
    let temp_file = temp_dir.join(format!("{:x}.json", sha.finalize()));
    let handle_http_error = |error: reqwest::Error| ProtoError::Http {
        url: url.to_owned(),
        error,
    };
    let offline = is_offline();

    if temp_file.exists() {
        let metadata = fs::metadata(&temp_file)?;

        // When offline, always read the temp file as we can't download the manifest
        let read_temp = if offline {
            true
            // Otherwise, only read the temp file if its been downloaded in the last 24 hours
        } else if let Ok(modified_time) = metadata.modified().or_else(|_| metadata.created()) {
            modified_time > SystemTime::now() - Duration::from_secs(60 * 60 * 24)
        } else {
            false
        };

        if read_temp {
            trace!(
                "Loading versions manifest from locally cached {}",
                color::path(&temp_file),
            );

            let contents: T = json::read_file(&temp_file)?;

            return Ok(contents);
        }
    }

    if offline {
        return Err(ProtoError::InternetConnectionRequired);
    }

    // Otherwise, request the resource and cache it
    trace!("Loading versions manifest from {}", color::url(url));

    let response = reqwest::get(url).await.map_err(handle_http_error)?;
    let contents = response.text().await.map_err(handle_http_error)?;

    fs::create_dir_all(&temp_dir)?;
    fs::write_file(&temp_file, &contents)?;

    let data: T = serde_json::from_str(&contents).map_err(|error| JsonError::ReadFile {
        path: temp_file.to_path_buf(),
        error,
    })?;

    Ok(data)
}

pub fn parse_version(version: &str) -> Result<Version, ProtoError> {
    Version::parse(version).map_err(|error| ProtoError::SemverLenient {
        version: version.to_owned(),
        error: error.owned(),
    })
}

pub fn is_semantic_version(version: &str) -> bool {
    Version::parse(version).is_ok()
}
