use crate::workflows::InstallPhaseReporter;
use iocraft::prelude::*;
use proto_core::flow::install::InstallPhase;
use starbase_console::ui::*;

#[derive(Default, Props)]
pub struct InstallProgressProps {
    pub phase_reporter: InstallPhaseReporter,
    pub progress_reporter: ProgressReporter,
}

#[component]
pub fn InstallProgress<'a>(
    props: &InstallProgressProps,
    mut hooks: Hooks,
) -> impl Into<AnyElement<'a>> {
    let mut phase = hooks.use_state(|| InstallPhase::Download);
    let phase_reporter = props.phase_reporter.clone();
    let progress_reporter = props.progress_reporter.clone();

    hooks.use_future(async move {
        let mut receiver = phase_reporter.subscribe();

        while let Ok(next_phase) = receiver.recv().await {
            phase.set(next_phase);
        }
    });

    element! {
        Box {
            #(if matches!(phase.get(), InstallPhase::Download) {
                element! {
                    Box {
                        ProgressBar(
                            default_message: "Preparing install...",
                            bar_width: 20u32, // Width of loader frames
                            reporter: progress_reporter,
                        )
                    }
                }
            } else {
                element! {
                    Box {
                        ProgressLoader(
                            reporter: progress_reporter
                        )
                    }
                }
            })
        }
    }
}
