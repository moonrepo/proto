use miette::IntoDiagnostic;
use starbase_console::ui::ProgressReporter;
use std::ops::Deref;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct ProgressInstance {
    pub handle: JoinHandle<miette::Result<()>>,
    pub reporter: Arc<ProgressReporter>,
}

impl ProgressInstance {
    pub async fn stop(self) -> miette::Result<()> {
        self.reporter.exit();
        self.handle.await.into_diagnostic()??;

        Ok(())
    }
}

impl Deref for ProgressInstance {
    type Target = ProgressReporter;

    fn deref(&self) -> &Self::Target {
        &self.reporter
    }
}
