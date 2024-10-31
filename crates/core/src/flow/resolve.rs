use crate::error::ProtoError;
use crate::helpers::is_offline;
use crate::tool::Tool;
use crate::version_resolver::VersionResolver;
use proto_pdk_api::*;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, trace};

impl Tool {
    /// Load available versions to install and return a resolver instance.
    /// To reduce network overhead, results will be cached for 24 hours.
    #[instrument(skip(self))]
    pub async fn load_version_resolver(
        &self,
        initial_version: &UnresolvedVersionSpec,
    ) -> miette::Result<VersionResolver> {
        debug!(tool = self.id.as_str(), "Loading available versions");

        let mut versions = LoadVersionsOutput::default();
        let mut cached = false;

        if let Some(cached_versions) = self.inventory.load_remote_versions(!self.cache)? {
            versions = cached_versions;
            cached = true;
        }

        // Nothing cached, so load from the plugin
        if !cached {
            if is_offline() {
                return Err(ProtoError::InternetConnectionRequiredForVersion {
                    command: format!("{}_VERSION=1.2.3 {}", self.get_env_var_prefix(), self.id),
                    bin_dir: self.proto.store.bin_dir.clone(),
                }
                .into());
            }

            if env::var("PROTO_BYPASS_VERSION_CHECK").is_err() {
                versions = self
                    .plugin
                    .cache_func_with(
                        "load_versions",
                        LoadVersionsInput {
                            initial: initial_version.to_owned(),
                        },
                    )
                    .await?;

                self.inventory.save_remote_versions(&versions)?;
            }
        }

        // Cache the results and create a resolver
        let mut resolver = VersionResolver::from_output(versions);

        resolver.with_manifest(&self.inventory.manifest);

        let config = self.proto.load_config()?;

        if let Some(tool_config) = config.tools.get(&self.id) {
            resolver.with_config(tool_config);
        }

        Ok(resolver)
    }

    /// Given an initial version, resolve it to a fully qualifed and semantic version
    /// (or alias) according to the tool's ecosystem.
    #[instrument(skip(self))]
    pub async fn resolve_version(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
        short_circuit: bool,
    ) -> miette::Result<VersionSpec> {
        if self.version.is_some() {
            return Ok(self.get_resolved_version());
        }

        debug!(
            tool = self.id.as_str(),
            initial_version = initial_version.to_string(),
            "Resolving a semantic version or alias",
        );

        // If we have a fully qualified semantic version,
        // exit early and assume the version is legitimate!
        // Also canary is a special type that we can simply just use.
        if short_circuit
            && matches!(
                initial_version,
                UnresolvedVersionSpec::Calendar(_) | UnresolvedVersionSpec::Semantic(_)
            )
            || matches!(initial_version, UnresolvedVersionSpec::Canary)
        {
            let version = initial_version.to_resolved_spec();

            debug!(
                tool = self.id.as_str(),
                version = version.to_string(),
                "Resolved to {} (without validation)",
                version
            );

            self.set_version(version.clone());

            return Ok(version);
        }

        let resolver = self.load_version_resolver(initial_version).await?;
        let version = self
            .resolve_version_candidate(&resolver, initial_version, true)
            .await?;

        debug!(
            tool = self.id.as_str(),
            version = version.to_string(),
            "Resolved to {}",
            version
        );

        self.set_version(version.clone());

        Ok(version)
    }

    #[instrument(skip(self, resolver))]
    pub async fn resolve_version_candidate(
        &self,
        resolver: &VersionResolver<'_>,
        initial_candidate: &UnresolvedVersionSpec,
        with_manifest: bool,
    ) -> miette::Result<VersionSpec> {
        let resolve = |candidate: &UnresolvedVersionSpec| {
            let result = if with_manifest {
                resolver.resolve(candidate)
            } else {
                resolver.resolve_without_manifest(candidate)
            };

            result.ok_or_else(|| ProtoError::VersionResolveFailed {
                tool: self.get_name().to_owned(),
                version: candidate.to_string(),
            })
        };

        if self.plugin.has_func("resolve_version").await {
            let output: ResolveVersionOutput = self
                .plugin
                .call_func_with(
                    "resolve_version",
                    ResolveVersionInput {
                        initial: initial_candidate.to_owned(),
                    },
                )
                .await?;

            if let Some(candidate) = output.candidate {
                debug!(
                    tool = self.id.as_str(),
                    candidate = candidate.to_string(),
                    "Received a possible version or alias to use",
                );

                return Ok(resolve(&candidate)?);
            }

            if let Some(candidate) = output.version {
                debug!(
                    tool = self.id.as_str(),
                    version = candidate.to_string(),
                    "Received an explicit version or alias to use",
                );

                return Ok(candidate);
            }
        }

        Ok(resolve(initial_candidate)?)
    }

    /// Attempt to detect an applicable version from the provided directory.
    #[instrument(skip(self))]
    pub async fn detect_version_from(
        &self,
        current_dir: &Path,
    ) -> miette::Result<Option<(UnresolvedVersionSpec, PathBuf)>> {
        if !self.plugin.has_func("detect_version_files").await {
            return Ok(None);
        }

        let has_parser = self.plugin.has_func("parse_version_file").await;
        let output: DetectVersionOutput = self.plugin.cache_func("detect_version_files").await?;

        if !output.ignore.is_empty() {
            if let Some(dir) = current_dir.to_str() {
                if output.ignore.iter().any(|ignore| dir.contains(ignore)) {
                    return Ok(None);
                }
            }
        }

        trace!(
            tool = self.id.as_str(),
            dir = ?current_dir,
            "Attempting to detect a version from directory"
        );

        for file in output.files {
            let file_path = current_dir.join(&file);

            if !file_path.exists() {
                continue;
            }

            let content = fs::read_file(&file_path)?.trim().to_owned();

            if content.is_empty() {
                continue;
            }

            let version = if has_parser {
                let output: ParseVersionFileOutput = self
                    .plugin
                    .call_func_with(
                        "parse_version_file",
                        ParseVersionFileInput {
                            content,
                            file: file.clone(),
                            path: self.to_virtual_path(&file_path),
                        },
                    )
                    .await?;

                if output.version.is_none() {
                    continue;
                }

                output.version.unwrap()
            } else {
                UnresolvedVersionSpec::parse(&content).map_err(|error| ProtoError::VersionSpec {
                    version: content,
                    error: Box::new(error),
                })?
            };

            debug!(
                tool = self.id.as_str(),
                file = ?file_path,
                version = version.to_string(),
                "Detected a version"
            );

            return Ok(Some((version, file_path)));
        }

        Ok(None)
    }
}
