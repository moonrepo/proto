use super::tool_record::ToolRecord;
use proto_core::{ProtoToolError, ToolContext};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

#[derive(Debug, PartialEq)]
pub enum InstallStatus {
    ReqFailed(ToolContext),
    WaitingOnReqs(Vec<ToolContext>),
    Waiting,
}

#[derive(Clone)]
pub struct InstallGraph {
    /// Tools that must be installed
    contexts: Arc<FxHashSet<ToolContext>>,

    /// Tools that have been installed
    installed: Arc<RwLock<FxHashSet<ToolContext>>>,

    /// Tools that have been not installed (some error occurred)
    not_installed: Arc<RwLock<FxHashSet<ToolContext>>>,

    /// Tools that require other tools to be installed
    /// before they can be installed
    requires: Arc<FxHashMap<ToolContext, Vec<ToolContext>>>,

    /// We should wait to install all tools until
    /// this boolean is turned off
    waiting: Arc<AtomicBool>,
}

impl InstallGraph {
    pub fn new(tools: &[ToolRecord]) -> Result<Self, ProtoToolError> {
        let mut items = vec![];

        for tool in tools {
            items.push((tool.context.clone(), tool.get_requires()?));
        }

        Ok(Self::from_requires(items))
    }

    pub fn from_requires(items: Vec<(ToolContext, Vec<ToolContext>)>) -> Self {
        let mut contexts = FxHashSet::default();
        let mut requires = FxHashMap::default();

        for (context, require_contexts) in items {
            contexts.insert(context.clone());

            if !require_contexts.is_empty() {
                requires.insert(context, require_contexts);
            }
        }

        Self {
            contexts: Arc::new(contexts),
            installed: Arc::new(RwLock::new(FxHashSet::default())),
            not_installed: Arc::new(RwLock::new(FxHashSet::default())),
            requires: Arc::new(requires),
            waiting: Arc::new(AtomicBool::new(true)),
        }
    }

    pub async fn check_install_status(&self, context: &ToolContext) -> Option<InstallStatus> {
        if self.waiting.load(Ordering::Relaxed) {
            return Some(InstallStatus::Waiting);
        }

        if !self.contexts.contains(context) {
            return None;
        }

        if let Some(require_contexts) = self.requires.get(context) {
            let installed = self.installed.read().await;
            let not_installed = self.not_installed.read().await;
            let mut waiting_on = vec![];

            for require_context in require_contexts {
                if !self.contexts.contains(require_context) {
                    continue;
                }

                if not_installed.contains(require_context) {
                    return Some(InstallStatus::ReqFailed(require_context.clone()));
                }

                if !installed.contains(require_context) {
                    waiting_on.push(require_context.clone());
                }
            }

            if !waiting_on.is_empty() {
                return Some(InstallStatus::WaitingOnReqs(waiting_on));
            }
        }

        None
    }

    pub async fn mark_installed(&self, context: &ToolContext) {
        self.installed.write().await.insert(context.to_owned());
    }

    pub async fn mark_not_installed(&self, context: &ToolContext) {
        self.not_installed.write().await.insert(context.to_owned());
    }

    pub fn proceed(&mut self) {
        self.waiting.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(value: &str) -> ToolContext {
        ToolContext::parse(value).unwrap()
    }

    fn create_graph(items: &[(&str, &[&str])]) -> InstallGraph {
        let mut graph = InstallGraph::from_requires(
            items
                .iter()
                .map(|(context, requires)| {
                    (ctx(context), requires.iter().map(|r| ctx(r)).collect())
                })
                .collect(),
        );
        graph.proceed();
        graph
    }

    #[tokio::test]
    async fn waits_until_proceeding() {
        let graph = InstallGraph::from_requires(vec![(ctx("node"), vec![])]);

        assert_eq!(
            graph.check_install_status(&ctx("node")).await,
            Some(InstallStatus::Waiting)
        );
    }

    #[tokio::test]
    async fn waits_on_backend_requirements() {
        let graph = create_graph(&[("npm:typescript", &["npm:tsx"]), ("npm:tsx", &[])]);

        assert_eq!(
            graph.check_install_status(&ctx("npm:typescript")).await,
            Some(InstallStatus::WaitingOnReqs(vec![ctx("npm:tsx")]))
        );
        assert_eq!(graph.check_install_status(&ctx("npm:tsx")).await, None);

        graph.mark_installed(&ctx("npm:tsx")).await;

        assert_eq!(
            graph.check_install_status(&ctx("npm:typescript")).await,
            None
        );
    }

    #[tokio::test]
    async fn doesnt_match_requirement_by_id_only() {
        // `tsx` and `npm:tsx` are different tools
        let graph = create_graph(&[("npm:typescript", &["npm:tsx"]), ("tsx", &[])]);

        assert_eq!(
            graph.check_install_status(&ctx("npm:typescript")).await,
            None
        );
    }

    #[tokio::test]
    async fn fails_if_requirement_failed() {
        let graph = create_graph(&[("npm", &["node"]), ("node", &[])]);

        graph.mark_not_installed(&ctx("node")).await;

        assert_eq!(
            graph.check_install_status(&ctx("npm")).await,
            Some(InstallStatus::ReqFailed(ctx("node")))
        );
    }

    #[tokio::test]
    async fn ignores_requirements_not_being_installed() {
        let graph = create_graph(&[("npm", &["node"])]);

        assert_eq!(graph.check_install_status(&ctx("npm")).await, None);
    }
}
