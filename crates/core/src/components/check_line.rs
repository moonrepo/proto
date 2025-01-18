use iocraft::prelude::*;
use starbase_console::ui::*;

#[derive(Default, Props)]
pub struct CheckLineProps {
    pub message: String,
    pub passed: bool,
}

#[component]
pub fn CheckLine<'a>(props: &mut CheckLineProps, hooks: Hooks) -> impl Into<AnyElement<'a>> {
    let theme = hooks.use_context::<ConsoleTheme>();

    element! {
        Group(gap: 1) {
            #(if props.passed {
                element!(StyledText(
                    content: &theme.form_success_symbol,
                    style: Style::Success
                ))
            } else {
                element!(StyledText(
                    content: &theme.form_failure_symbol,
                    style: Style::Failure
                ))
            })

            StyledText(
                content: &props.message,
            )
        }
    }
}
