use super::is_path_like;
use iocraft::prelude::*;
use proto_core::EnvVar as EnvVarConfig;
use starbase_console::ui::{Style, StyledText};

#[derive(Default, Props)]
pub struct EnvVarProps<'a> {
    pub value: Option<&'a EnvVarConfig>,
}

#[component]
pub fn EnvVar<'a>(props: &EnvVarProps<'a>) -> impl Into<AnyElement<'a>> {
    match props.value.as_ref().expect("`value` prop is required!") {
        EnvVarConfig::State(state) => {
            if *state {
                element! {
                    StyledText(
                        content: "true",
                        style: Style::Symbol
                    )
                }
            } else {
                element! {
                    StyledText(
                        content: "(removed)",
                        style: Style::Caution
                    )
                }
            }
        }
        EnvVarConfig::Value(value) => {
            element! {
                StyledText(
                    content: value,
                    style: if is_path_like(value) {
                        Style::Path
                    } else {
                        Style::MutedLight
                    }
                )
            }
        }
    }
}
