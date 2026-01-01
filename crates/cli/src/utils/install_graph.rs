use super::tool_record::ToolRecord;
use proto_core::Id;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

#[derive(Debug)]
pub enum InstallStatus {
    ReqFailed(Id),
    WaitingOnReqs(Vec<Id>),
    Waiting,
}

#[derive(Clone)]
pub struct InstallGraph {
    /// Tools that must be installed
    ids: Arc<FxHashSet<Id>>,

    /// Tools that have been installed
    installed: Arc<RwLock<FxHashSet<Id>>>,

    /// Tools that have been not installed (some error occurred)
    not_installed: Arc<RwLock<FxHashSet<Id>>>,

    /// Tools that require other tools to be installed
    /// before they can be installed
    requires: Arc<FxHashMap<Id, Vec<Id>>>,

    /// We should wait to install all tools until
    /// this boolean is turned off
    waiting: Arc<AtomicBool>,
}

impl InstallGraph {
    pub fn new(tools: &[ToolRecord]) -> Self {
        let mut ids = FxHashSet::default();
        let mut requires = FxHashMap::default();

        for tool in tools {
            ids.insert(tool.get_id().clone());

            if !tool.metadata.requires.is_empty() {
                requires.insert(
                    tool.get_id().clone(),
                    tool.metadata.requires.iter().map(Id::raw).collect(),
                );
            }
        }

        Self {
            ids: Arc::new(ids),
            installed: Arc::new(RwLock::new(FxHashSet::default())),
            not_installed: Arc::new(RwLock::new(FxHashSet::default())),
            requires: Arc::new(requires),
            waiting: Arc::new(AtomicBool::new(true)),
        }
    }

    pub async fn check_install_status(&self, id: &Id) -> Option<InstallStatus> {
        if self.waiting.load(Ordering::Relaxed) {
            return Some(InstallStatus::Waiting);
        }

        if !self.ids.contains(id) {
            return None;
        }

        if let Some(require_ids) = self.requires.get(id) {
            let installed = self.installed.read().await;
            let not_installed = self.not_installed.read().await;
            let mut waiting_on = vec![];

            for require_id in require_ids {
                if !self.ids.contains(require_id) {
                    continue;
                }

                if !installed.contains(require_id) {
                    waiting_on.push(require_id.clone());
                } else if not_installed.contains(require_id) {
                    return Some(InstallStatus::ReqFailed(require_id.clone()));
                }
            }

            if !waiting_on.is_empty() {
                return Some(InstallStatus::WaitingOnReqs(waiting_on));
            }
        }

        None
    }

    pub async fn mark_installed(&self, id: &Id) {
        self.installed.write().await.insert(id.to_owned());
    }

    pub async fn mark_not_installed(&self, id: &Id) {
        self.not_installed.write().await.insert(id.to_owned());
    }

    pub fn proceed(&mut self) {
        self.waiting.store(false, Ordering::Release);
    }
}
