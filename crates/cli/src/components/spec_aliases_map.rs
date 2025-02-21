use iocraft::prelude::*;
use proto_core::ToolSpec;
use starbase_console::ui::{Map, MapItem, Style, StyledText};
use std::collections::BTreeMap;

#[derive(Default, Props)]
pub struct SpecAliasesMapProps<'a> {
    pub aliases: BTreeMap<&'a String, &'a ToolSpec>,
}

#[component]
pub fn SpecAliasesMap<'a>(props: &SpecAliasesMapProps<'a>) -> impl Into<AnyElement<'a>> + use<'a> {
    element! {
        Map {
            #(props.aliases.iter().map(|(alias, version)| {
                element! {
                    MapItem(
                        name: element! {
                            StyledText(
                                content: alias.to_owned(),
                                style: Style::Id
                            )
                        }.into_any(),
                        value: element! {
                            StyledText(
                                content: version.to_string(),
                                style: Style::Invalid
                            )
                        }.into_any()
                    )
                }
            }))
        }
    }
}
