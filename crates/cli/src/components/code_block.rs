use iocraft::prelude::*;
use starbase_console::ui::{Style, StyledText};

#[derive(Default, Props)]
pub struct CodeBlockProps {
    pub code: String,
    pub format: String,
}

#[component]
pub fn CodeBlock<'a>(props: &CodeBlockProps) -> impl Into<AnyElement<'a>> {
    element! {
        Box(
            flex_direction: FlexDirection::Column,
            padding_left: 2,
            padding_top: 1,
            padding_bottom: 1,
        ) {
            #(props.code.lines().map(|line| {
                element! {
                    StyledText(
                        content: line,
                        style: if props.format == "toml" && line.starts_with('[') {
                            None
                        } else {
                            Some(Style::MutedLight)
                        }
                    )
                }
            }))
        }
    }
}
