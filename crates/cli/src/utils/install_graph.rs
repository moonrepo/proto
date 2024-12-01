use proto_core::{Id, Tool};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct InstallGraph {
    /// Tools that must be installed
    ids: Arc<FxHashSet<Id>>,

    /// Tools that have been installed
    installed: Arc<RwLock<FxHashSet<Id>>>,

    /// Tools that require other tools to be installed
    /// before they can be installed
    requires: Arc<FxHashMap<Id, Vec<Id>>>,
}

impl InstallGraph {
    pub fn new(tools: &[Tool]) -> Self {
        let mut ids = FxHashSet::default();
        let mut requires = FxHashMap::default();

        for tool in tools {
            ids.insert(tool.id.clone());

            if !tool.metadata.requires.is_empty() {
                requires.insert(
                    tool.id.clone(),
                    tool.metadata.requires.iter().map(Id::raw).collect(),
                );
            }
        }

        Self {
            ids: Arc::new(ids),
            installed: Arc::new(RwLock::new(FxHashSet::default())),
            requires: Arc::new(requires),
        }
    }

    pub async fn can_install(&self, id: &Id) -> bool {
        if !self.ids.contains(id) {
            return false;
        }

        if let Some(require_ids) = self.requires.get(id) {
            let installed = self.installed.read().await;

            for require_id in require_ids {
                if self.ids.contains(require_id) && !installed.contains(require_id) {
                    return false;
                }
            }
        }

        true
    }

    pub async fn mark_installed(&self, id: &Id) {
        self.installed.write().await.insert(id.to_owned());
    }
}
