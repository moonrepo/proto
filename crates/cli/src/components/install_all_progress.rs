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
            match state {
                ProgressState::Exit => {
                    should_exit.set(true);
                    break;
                }
                _ => {}
            }
        }
    });

    if should_exit.get() {
        system.exit();

        return element!(Box).into_any();
    }

    element! {
        Stack {
            #(props.tools.iter().map(|(id, inner_props)| {
                element! {
                    Box {
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
