pub use super::resolve_error::ProtoResolveError;
use crate::flow::lock::Locker;
use crate::helpers::is_offline;
use crate::tool::Tool;
use crate::tool_spec::ToolSpec;
use crate::version_resolver::VersionResolver;
use proto_pdk_api::*;
use std::env;
use tracing::{debug, instrument};

/// Loads, resolves, and validates versions.
pub struct Resolver<'tool> {
    tool: &'tool Tool,

    /// Collection of loaded versions.
    pub data: VersionResolver<'tool>,
}

impl<'tool> Resolver<'tool> {
    pub fn new(tool: &'tool Tool) -> Self {
        Self {
            tool,
            data: VersionResolver::default(),
        }
    }

    #[instrument]
    pub async fn resolve(
        tool: &'tool Tool,
        spec: &mut ToolSpec,
        short_circuit: bool,
    ) -> Result<VersionSpec, ProtoResolveError> {
        Self::new(tool).resolve_version(spec, short_circuit).await
    }

    /// Load available versions to install and return a resolver instance.
    /// To reduce network overhead, results will be cached for 12 hours.
    #[instrument(skip(self))]
    pub async fn load_versions(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
    ) -> Result<(), ProtoResolveError> {
        debug!(
            tool = self.tool.context.as_str(),
            "Loading available versions"
        );

        let mut versions = LoadVersionsOutput::default();
        let mut cached = false;

        if let Some(cached_versions) = self
            .tool
            .inventory
            .load_remote_versions(!self.tool.cache, initial_version.get_scope())?
        {
            versions = cached_versions;
            cached = true;
        }

        // Nothing cached, so load from the plugin
        if !cached {
            if is_offline() {
                return Err(ProtoResolveError::RequiredInternetConnectionForVersion {
                    command: format!(
                        "{}_VERSION=1.2.3 {}",
                        self.tool.get_env_var_prefix(),
                        self.tool.get_id()
                    ),
                    bin_dir: self.tool.proto.store.bin_dir.clone(),
                });
            }

            if env::var("PROTO_BYPASS_VERSION_CHECK").is_err() {
                versions = self
                    .tool
                    .plugin
                    .cache_func_with(
                        PluginFunction::LoadVersions,
                        LoadVersionsInput {
                            context: self.tool.create_plugin_unresolved_context(),
                            initial: initial_version.to_owned(),
                        },
                    )
                    .await?;

                if !versions.versions.is_empty() {
                    self.tool
                        .inventory
                        .save_remote_versions(&versions, initial_version.get_scope())?;
                }
            }
        }

        // Cache the results and create a resolver
        let mut resolver = VersionResolver::from_output(versions);

        resolver.with_manifest(&self.tool.inventory.manifest);

        let config = self.tool.proto.load_config()?;

        if let Some(tool_config) = config.get_tool_config(&self.tool.context) {
            resolver.with_config(tool_config);
        }

        self.data = resolver;

        Ok(())
    }

    /// Given an initial spec, resolve it to a fully qualifed and semantic version
    /// (or alias) according to the tool's ecosystem.
    #[instrument(skip(self))]
    pub async fn resolve_version(
        &mut self,
        spec: &mut ToolSpec,
        short_circuit: bool,
    ) -> Result<VersionSpec, ProtoResolveError> {
        if spec.is_resolved() {
            return Ok(spec.get_resolved_version());
        }

        debug!(
            tool = self.tool.context.as_str(),
            spec = spec.to_string(),
            "Resolving a semantic version or alias",
        );

        let mut candidate = spec.req.clone();

        // If requested, resolve the version from a lockfile
        if spec.resolve_from_lockfile
            && let Some(record) = Locker::new(self.tool).resolve_locked_record(spec)?
        {
            let version = record
                .version
                .clone()
                .expect("Version missing from lockfile record!");

            debug!(
                tool = self.tool.context.as_str(),
                spec = candidate.to_string(),
                "Inherited version {} from lockfile",
                version
            );

            spec.version_locked = Some(record);
            candidate = version.to_unresolved_spec();
        }

        let version = self
            .resolve_version_candidate(&candidate, short_circuit, spec.resolve_from_manifest)
            .await?;

        debug!(
            tool = self.tool.context.as_str(),
            spec = candidate.to_string(),
            "Resolved to {}",
            version
        );

        spec.resolve(version.clone());

        Ok(version)
    }

    #[instrument(skip(self))]
    pub async fn resolve_version_candidate(
        &mut self,
        candidate: &UnresolvedVersionSpec,
        short_circuit: bool,
        resolve_from_manifest: bool,
    ) -> Result<VersionSpec, ProtoResolveError> {
        let mut candidate = candidate.to_owned();

        // If we have a fully qualified semantic version,
        // exit early and assume the version is legitimate!
        // Also canary is a special type that we can simply just use.
        if (short_circuit && candidate.is_fully_qualified())
            || matches!(candidate, UnresolvedVersionSpec::Canary)
        {
            let version = candidate.to_resolved_spec();

            debug!(
                tool = self.tool.context.as_str(),
                spec = candidate.to_string(),
                "Resolved to {} (without validation)",
                version
            );

            return Ok(version);
        }

        // Resolve the version from the plugin if it has a custom resolver,
        // as we need to inherit any custom scopes for caching
        let mut version = None;

        if self
            .tool
            .plugin
            .has_func(PluginFunction::ResolveVersion)
            .await
        {
            let output: ResolveVersionOutput = self
                .tool
                .plugin
                .call_func_with(
                    PluginFunction::ResolveVersion,
                    ResolveVersionInput {
                        context: self.tool.create_plugin_unresolved_context(),
                        initial: candidate.to_owned(),
                    },
                )
                .await?;

            if let Some(new_candidate) = output.candidate {
                debug!(
                    tool = self.tool.context.as_str(),
                    candidate = new_candidate.to_string(),
                    "Received a possible version or alias to use",
                );

                candidate = new_candidate;
            }

            if let Some(new_version) = output.version {
                debug!(
                    tool = self.tool.context.as_str(),
                    version = new_version.to_string(),
                    "Received an explicit version or alias to use",
                );

                version = Some(new_version);
            }
        }

        if version.is_none() {
            self.load_versions(&candidate).await?;

            version = self
                .resolve_version_from_list(&candidate, resolve_from_manifest)
                .await;
        }

        version.ok_or_else(|| ProtoResolveError::FailedVersionResolve {
            tool: self.tool.get_name().to_owned(),
            version: candidate.to_string(),
        })
    }

    /// Given a list of version candidates, resolve one to a valid version by
    /// calling the plugin to validate and choose.
    #[instrument(skip(self))]
    pub async fn resolve_version_from_list(
        &self,
        candidate: &UnresolvedVersionSpec,
        with_manifest: bool,
    ) -> Option<VersionSpec> {
        if with_manifest {
            self.data.resolve(candidate)
        } else {
            self.data.resolve_without_manifest(candidate)
        }
    }
}
