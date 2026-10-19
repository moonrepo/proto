use crate::error::ProtoCliError;
use indexmap::IndexSet;
use proto_core::flow::detect::Detector;
use proto_core::flow::resolve::{ProtoResolveError, Resolver};
use proto_core::{
    ProtoConfig, ProtoToolConfig, Tool, ToolContext, ToolSpec, UnresolvedVersionSpec, VersionSpec,
};
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

#[derive(Debug)]
pub struct ToolRecord {
    pub tool: Tool,
    pub spec: ToolSpec,
    pub config: ProtoToolConfig,
    pub detected_source: Option<PathBuf>,
    pub detected_version: Option<ToolSpec>,
    pub installed_versions: Vec<VersionSpec>,
    pub local_aliases: BTreeMap<String, ToolSpec>,
    pub remote_aliases: BTreeMap<String, ToolSpec>,
    pub remote_versions: Vec<VersionSpec>,
}

impl ToolRecord {
    pub fn new(tool: Tool) -> Self {
        let mut versions = tool
            .inventory
            .manifest
            .installed_versions
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        versions.sort();

        Self {
            tool,
            spec: ToolSpec::parse("*").unwrap(),
            config: ProtoToolConfig::default(),
            detected_source: None,
            detected_version: None,
            local_aliases: BTreeMap::default(),
            remote_aliases: BTreeMap::default(),
            installed_versions: versions,
            remote_versions: vec![],
        }
    }

    pub async fn detect_version_and_source(&mut self) {
        if let Ok((config_version, source)) = Detector::detect(&self.tool).await {
            self.detected_version = Some(config_version);
            self.detected_source = source;
        }
    }

    pub fn inherit_from_local(&mut self, config: &ProtoConfig) {
        if let Some(tool_config) = config.get_tool_config(&self.context).cloned() {
            self.local_aliases.extend(tool_config.aliases.clone());
            self.config = tool_config;
        }
    }

    pub async fn inherit_from_remote(
        &mut self,
        spec: &UnresolvedVersionSpec,
    ) -> Result<(), ProtoResolveError> {
        let mut resolver = Resolver::new(&self.tool);
        resolver.load_versions(spec).await?;

        self.remote_aliases.extend(
            resolver
                .data
                .aliases
                .into_iter()
                .map(|(k, v)| (k, ToolSpec::new(v))),
        );
        self.remote_versions.extend(resolver.data.versions);
        self.remote_versions.sort();

        Ok(())
    }
}

impl Deref for ToolRecord {
    type Target = Tool;

    fn deref(&self) -> &Self::Target {
        &self.tool
    }
}

impl DerefMut for ToolRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tool
    }
}

/// Given a list of `(context, requires)` pairs, return the contexts in
/// dependency-first order (topological sort). Dependencies come before
/// the tools that require them. Errors if the requirements form a cycle.
pub fn sort_contexts_by_dependency(
    items: &[(ToolContext, Vec<ToolContext>)],
) -> Result<IndexSet<ToolContext>, ProtoCliError> {
    let mut visiting = vec![];
    let mut visited = IndexSet::default();

    fn visit(
        context: &ToolContext,
        items: &[(ToolContext, Vec<ToolContext>)],
        visiting: &mut Vec<ToolContext>,
        visited: &mut IndexSet<ToolContext>,
    ) -> Result<(), ProtoCliError> {
        if visited.contains(context) {
            return Ok(());
        }

        let Some((_, requires)) = items.iter().find(|(ctx, _)| ctx == context) else {
            return Ok(());
        };

        if let Some(index) = visiting.iter().position(|ctx| ctx == context) {
            let mut cycle = visiting[index..]
                .iter()
                .map(|ctx| ctx.to_string())
                .collect::<Vec<_>>();
            cycle.push(context.to_string());

            return Err(ProtoCliError::ToolRequirementsCycle {
                cycle: cycle.join(" → "),
            });
        }

        visiting.push(context.clone());

        for dependency in requires {
            visit(dependency, items, visiting, visited)?;
        }

        visiting.pop();
        visited.insert(context.clone());

        Ok(())
    }

    for (context, _) in items {
        visit(context, items, &mut visiting, &mut visited)?;
    }

    Ok(visited)
}

pub fn sort_tools_by_dependency(
    mut tools: Vec<ToolRecord>,
) -> Result<Vec<ToolRecord>, ProtoCliError> {
    let mut items = vec![];

    for tool in &tools {
        items.push((tool.context.clone(), tool.get_requires()?));
    }

    let sorted = sort_contexts_by_dependency(&items)?;
    let mut list = vec![];

    for context in sorted {
        if let Some(index) = tools.iter().position(|tool| tool.context == context) {
            list.push(tools.remove(index));
        }
    }

    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(s: &str) -> ToolContext {
        ToolContext::parse(s).unwrap()
    }

    fn try_sort(items: &[(&str, &[&str])]) -> Result<Vec<String>, ProtoCliError> {
        let owned: Vec<(ToolContext, Vec<ToolContext>)> = items
            .iter()
            .map(|(name, deps)| (ctx(name), deps.iter().map(|d| ctx(d)).collect()))
            .collect();

        let visited = sort_contexts_by_dependency(&owned)?;

        // No reversal — dependency-first (topological) order
        Ok(visited.into_iter().map(|c| c.to_string()).collect())
    }

    fn sort(items: &[(&str, &[&str])]) -> Vec<String> {
        try_sort(items).unwrap()
    }

    fn sort_cycle(items: &[(&str, &[&str])]) -> String {
        match try_sort(items) {
            Err(ProtoCliError::ToolRequirementsCycle { cycle }) => cycle,
            other => panic!("expected a cycle, got {other:?}"),
        }
    }

    #[test]
    fn empty_input() {
        assert!(sort(&[]).is_empty());
    }

    #[test]
    fn single_tool_no_deps() {
        assert_eq!(sort(&[("node", &[])]), vec!["node"]);
    }

    #[test]
    fn multiple_tools_no_deps() {
        // With no deps, visited order = input order
        let result = sort(&[("node", &[]), ("bun", &[]), ("go", &[])]);
        assert_eq!(result, vec!["node", "bun", "go"]);
    }

    #[test]
    fn linear_chain_a_requires_b_requires_c() {
        // a→b→c: deps visited first, so order is c, b, a
        let result = sort(&[("a", &["b"]), ("b", &["c"]), ("c", &[])]);
        assert_eq!(result, vec!["c", "b", "a"]);
    }

    #[test]
    fn diamond_dependency() {
        // a→{b,c}, b→d, c has no deps, d has no deps
        // Visiting a: visit b first → visit d first → d, b, then c, then a
        let result = sort(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &[]), ("d", &[])]);
        assert_eq!(result, vec!["d", "b", "c", "a"]);
    }

    #[test]
    fn unknown_dep_is_skipped() {
        // "a" depends on "unknown" which is not in the list
        let result = sort(&[("a", &["unknown"]), ("b", &[])]);
        // "unknown" not found → skipped; a and b both visited in input order
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn dep_ordering_with_backend_context() {
        // "npm:typescript" depends on "node"
        let result = sort(&[("npm:typescript", &["node"]), ("node", &[])]);
        // node visited first as dep, then npm:typescript
        assert_eq!(result, vec!["node", "npm:typescript"]);
    }

    #[test]
    fn tool_not_in_list_referenced_as_dep() {
        // b depends on c, but c is not in the tool list
        let result = sort(&[("a", &[]), ("b", &["c"])]);
        // c is not found, so visit(c) is a no-op; a and b visited in input order
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn dep_ordering_with_backend_dependency() {
        // "npm:typescript" depends on "npm:tsx", which is not "tsx"
        let result = sort(&[
            ("npm:typescript", &["npm:tsx"]),
            ("tsx", &[]),
            ("npm:tsx", &[]),
        ]);
        assert_eq!(result, vec!["npm:tsx", "npm:typescript", "tsx"]);
    }

    #[test]
    fn errors_for_direct_cycle() {
        assert_eq!(sort_cycle(&[("a", &["b"]), ("b", &["a"])]), "a → b → a");
    }

    #[test]
    fn errors_for_indirect_cycle() {
        assert_eq!(
            sort_cycle(&[("x", &[]), ("a", &["b"]), ("b", &["c"]), ("c", &["b"])]),
            "b → c → b"
        );
    }

    #[test]
    fn errors_for_self_cycle() {
        assert_eq!(sort_cycle(&[("a", &["a"])]), "a → a");
    }

    #[test]
    fn diamond_is_not_a_cycle() {
        let result = sort(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &["d"]), ("d", &[])]);
        assert_eq!(result, vec!["d", "b", "c", "a"]);
    }

    #[test]
    fn dep_appears_before_dependent_in_input() {
        // deps come before dependents
        let result = sort(&[("base", &[]), ("app", &["base"])]);
        // base visited first (as dep of app), then app
        assert_eq!(result, vec!["base", "app"]);
    }
}
