use proto_core::reporter::ProtoConsole;
use starbase_console::ConsoleError;
use starbase_console::ui::{ProgressReporter, ProgressState};
use std::ops::Deref;
use tokio::task::JoinHandle;

pub struct ProgressInstance {
    pub handle: JoinHandle<Result<(), ConsoleError>>,
    pub reporter: ProgressReporter,
}

impl ProgressInstance {
    pub async fn stop(self) -> Result<(), ConsoleError> {
        self.reporter.exit();

        if let Ok(result) = self.handle.await {
            result?;
        }

        Ok(())
    }
}

impl Deref for ProgressInstance {
    type Target = ProgressReporter;

    fn deref(&self) -> &Self::Target {
        &self.reporter
    }
}

pub fn monitor_non_tty_progress(
    console: ProtoConsole,
    reporter: ProgressReporter,
    id: Option<String>,
) -> JoinHandle<Result<(), ConsoleError>> {
    tokio::spawn(Box::pin(async move {
        let mut receiver = reporter.subscribe();

        while let Ok(state) = receiver.recv().await {
            match state {
                ProgressState::Exit => {
                    break;
                }
                ProgressState::Message(message) if !console.out.is_quiet() => {
                    let _ = console.progress(message, id.clone());
                }
                _ => {}
            }
        }

        Ok(())
    }))
}
