use iocraft::prelude::*;
use starbase_console::ui::*;

#[derive(Default, Props)]
pub struct InstallProgressProps {
    pub reporter: Option<ProgressReporter>,
}

#[component]
pub fn InstallProgress<'a>(props: &mut InstallProgressProps) -> impl Into<AnyElement<'a>> {
    element! {
        Box {
            Progress(
                default_message: "Preparing install...",
                bar_width: 20u32, // Width of loader frames
                reporter: props.reporter.take(),
            )
        }
    }
}
