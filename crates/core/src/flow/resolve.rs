pub use super::resolve_error::ProtoResolveError;
use crate::helpers::is_offline;
use crate::tool::Tool;
use crate::tool_spec::ToolSpec;
use crate::version_resolver::VersionResolver;
use proto_pdk_api::*;
use std::env;
use tracing::{debug, instrument};

impl Tool {
    /// Load available versions to install and return a resolver instance.
    /// To reduce network overhead, results will be cached for 24 hours.
    #[instrument(skip(self))]
    pub async fn load_version_resolver(
        &self,
        initial_version: &UnresolvedVersionSpec,
    ) -> Result<VersionResolver<'_>, ProtoResolveError> {
        debug!(tool = self.context.as_str(), "Loading available versions");

        let mut versions = LoadVersionsOutput::default();
        let mut cached = false;

        if let Some(cached_versions) = self.inventory.load_remote_versions(!self.cache)? {
            versions = cached_versions;
            cached = true;
        }

        // Nothing cached, so load from the plugin
        if !cached {
            if is_offline() {
                return Err(ProtoResolveError::RequiredInternetConnectionForVersion {
                    command: format!(
                        "{}_VERSION=1.2.3 {}",
                        self.get_env_var_prefix(),
                        self.get_id()
                    ),
                    bin_dir: self.proto.store.bin_dir.clone(),
                });
            }

            if env::var("PROTO_BYPASS_VERSION_CHECK").is_err() {
                versions = self
                    .plugin
                    .cache_func_with(
                        PluginFunction::LoadVersions,
                        LoadVersionsInput {
                            context: self.create_plugin_unresolved_context(),
                            initial: initial_version.to_owned(),
                        },
                    )
                    .await?;

                if !versions.versions.is_empty() {
                    self.inventory.save_remote_versions(&versions)?;
                }
            }
        }

        // Cache the results and create a resolver
        let mut resolver = VersionResolver::from_output(versions);

        resolver.with_manifest(&self.inventory.manifest);

        let config = self.proto.load_config()?;

        if let Some(tool_config) = config.tools.get(self.get_id()) {
            resolver.with_config(tool_config);
        }

        Ok(resolver)
    }

    /// Given an initial spec, resolve it to a fully qualifed and semantic version
    /// (or alias) according to the tool's ecosystem.
    #[instrument(skip(self))]
    pub async fn resolve_version(
        &mut self,
        spec: &ToolSpec,
        short_circuit: bool,
    ) -> Result<VersionSpec, ProtoResolveError> {
        if self.version.is_some() {
            return Ok(self.get_resolved_version());
        }

        debug!(
            tool = self.context.as_str(),
            initial_version = spec.to_string(),
            "Resolving a semantic version or alias",
        );

        let mut candidate = spec.req.clone();

        // If requested, resolve the version from a lockfile
        if spec.read_lockfile
            && let Some(record) = self.resolve_locked_record(spec)?
        {
            let version = record
                .version
                .clone()
                .expect("Version missing from lockfile record!");

            debug!(
                tool = self.context.as_str(),
                spec = candidate.to_string(),
                "Inherited version {} from lockfile",
                version
            );

            self.version_locked = Some(record);
            candidate = version.to_unresolved_spec();
        }

        // If we have a fully qualified semantic version,
        // exit early and assume the version is legitimate!
        // Also canary is a special type that we can simply just use.
        if short_circuit
            && matches!(
                candidate,
                UnresolvedVersionSpec::Calendar(_) | UnresolvedVersionSpec::Semantic(_)
            )
            || matches!(candidate, UnresolvedVersionSpec::Canary)
        {
            let version = candidate.to_resolved_spec();

            debug!(
                tool = self.context.as_str(),
                spec = candidate.to_string(),
                "Resolved to {} (without validation)",
                version
            );

            self.set_version(version.clone());

            return Ok(version);
        }

        let resolver = self.load_version_resolver(&candidate).await?;
        let version = self
            .resolve_version_candidate(&resolver, &candidate, spec.resolve_from_manifest)
            .await?;

        debug!(
            tool = self.context.as_str(),
            spec = candidate.to_string(),
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
    ) -> Result<VersionSpec, ProtoResolveError> {
        let resolve = |candidate: &UnresolvedVersionSpec| {
            let result = if with_manifest {
                resolver.resolve(candidate)
            } else {
                resolver.resolve_without_manifest(candidate)
            };

            result.ok_or_else(|| ProtoResolveError::FailedVersionResolve {
                tool: self.get_name().to_owned(),
                version: candidate.to_string(),
            })
        };

        if self.plugin.has_func(PluginFunction::ResolveVersion).await {
            let output: ResolveVersionOutput = self
                .plugin
                .call_func_with(
                    PluginFunction::ResolveVersion,
                    ResolveVersionInput {
                        context: self.create_plugin_unresolved_context(),
                        initial: initial_candidate.to_owned(),
                    },
                )
                .await?;

            if let Some(candidate) = output.candidate {
                debug!(
                    tool = self.context.as_str(),
                    candidate = candidate.to_string(),
                    "Received a possible version or alias to use",
                );

                return resolve(&candidate);
            }

            if let Some(candidate) = output.version {
                debug!(
                    tool = self.context.as_str(),
                    version = candidate.to_string(),
                    "Received an explicit version or alias to use",
                );

                return Ok(candidate);
            }
        }

        resolve(initial_candidate)
    }
}
