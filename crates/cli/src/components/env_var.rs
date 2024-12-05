use super::is_path_like;
use iocraft::prelude::*;
use proto_core::EnvVar as EnvVarConfig;
use starbase_console::ui::{Style, StyledText};

#[derive(Default, Props)]
pub struct EnvVarProps {
    pub value: Option<EnvVarConfig>,
}

#[component]
pub fn EnvVar<'a>(props: &EnvVarProps) -> impl Into<AnyElement<'a>> {
    match props.value.as_ref().expect("`value` prop required!") {
        EnvVarConfig::State(state) => {
            if *state {
                element! {
                    StyledText(
                        content: "true",
                        style: Style::MutedLight
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
