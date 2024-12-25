use super::install_progress::{InstallProgress, InstallProgressProps};
use iocraft::prelude::*;
use proto_core::Id;
use starbase_console::ui::*;
use std::collections::BTreeMap;

#[derive(Default, Props)]
pub struct InstallAllProgressProps {
    pub reporter: Option<ProgressReporter>,
    pub tools: BTreeMap<Id, InstallProgressProps>,
    pub id_width: usize,
}

#[component]
pub fn InstallAllProgress<'a>(
    props: &mut InstallAllProgressProps,
    mut hooks: Hooks,
) -> impl Into<AnyElement<'a>> {
    let mut system = hooks.use_context_mut::<SystemContext>();
    let mut should_exit = hooks.use_state(|| false);
    let reporter = props.reporter.take();

    hooks.use_future(async move {
        let Some(reporter) = reporter else {
            return;
        };

        let mut receiver = reporter.subscribe();

        while let Ok(state) = receiver.recv().await {
            if let ProgressState::Exit = state {
                should_exit.set(true);
                break;
            }
        }
    });

    if should_exit.get() {
        system.exit();

        // Don't return an empty element so that the final
        // install progress is displayed to the user, otherwise
        // they don't know which failed and which succeeded
    }

    element! {
        Box(flex_direction: FlexDirection::Column, margin_top: 1) {
            #(props.tools.iter().map(|(id, inner_props)| {
                element! {
                    Box(key: id.to_string()) {
                        Box(
                            justify_content: JustifyContent::End,
                            width: props.id_width as u16,
                        ) {
                            StyledText(
                                content: id.as_str(),
                                style: Style::Id,
                            )
                        }
                        Box(margin_left: 1) {
                            InstallProgress(
                                default_message: inner_props.default_message.clone(),
                                reporter: inner_props.reporter.clone(),
                            )
                        }
                    }
                }
            }))
        }
    }
    .into_any()
}
