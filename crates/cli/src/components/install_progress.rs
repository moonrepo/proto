use iocraft::prelude::*;
use starbase_console::ui::*;

#[derive(Default, Props)]
pub struct InstallProgressProps {
    pub default_message: Option<String>,
    pub reporter: Option<OwnedOrShared<ProgressReporter>>,
}

#[component]
pub fn InstallProgress<'a>(props: &mut InstallProgressProps) -> impl Into<AnyElement<'a>> {
    element! {
        Box {
            Progress(
                default_message: props.default_message.clone()
                    .unwrap_or_else(|| "Preparing install…".into()),
                default_max: 0u64,
                default_value: 0u64,
                bar_width: 20u32, // Width of loader frames
                reporter: props.reporter.take(),
            )
        }
    }
}
