use iocraft::prelude::*;
use proto_core::flow::install::InstallPhase;
use starbase_console::ui::*;
use std::time::Duration;

#[derive(Default, Props)]
pub struct InstallProgressProps {
    pub reporter: ProgressReporter,
}

#[component]
pub fn InstallProgress<'a>(
    props: &InstallProgressProps,
    mut hooks: Hooks,
) -> impl Into<AnyElement<'a>> {
    let mut phase = hooks.use_state(|| InstallPhase::Download);
    let receiver = props.reporter.rx.clone();

    hooks.use_future(async move {
        loop {
            while let Ok(ProgressState::CustomInt(phase_no)) = receiver.recv_async().await {
                let next_phase = match phase_no {
                    0 => InstallPhase::Native,
                    1 => InstallPhase::Download,
                    2 => InstallPhase::Verify,
                    3 => InstallPhase::Unpack,
                    _ => return,
                };

                phase.set(next_phase);
            }
        }
    });

    element! {
        Box {
            #(if matches!(phase.get(), InstallPhase::Download) {
                element! {
                    ProgressBar(
                        default_message: "Preparing install...",
                        bar_width: 20u32, // Width of loader frames
                        reporter: props.reporter.clone(),
                    )
                }.into_any()
            } else {
                element! {
                    ProgressLoader(
                        tick_interval: Duration::from_millis(25),
                        reporter: props.reporter.clone()
                    )
                }.into_any()
            })
        }
    }
}
