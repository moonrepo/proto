use starbase_console::ConsoleError;
use starbase_console::ui::ProgressReporter;
use std::ops::Deref;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct ProgressInstance {
    pub handle: JoinHandle<Result<(), ConsoleError>>,
    pub reporter: Arc<ProgressReporter>,
}

impl ProgressInstance {
    pub async fn stop(self) -> Result<(), ConsoleError> {
        self.reporter.exit();
        self.handle.await.unwrap()?;

        Ok(())
    }
}

impl Deref for ProgressInstance {
    type Target = ProgressReporter;

    fn deref(&self) -> &Self::Target {
        &self.reporter
    }
}
