use iocraft::prelude::*;
use proto_core::UnresolvedVersionSpec;
use starbase_console::ui::{Map, MapItem, Style, StyledText};
use std::collections::BTreeMap;

#[derive(Default, Props)]
pub struct AliasesMapProps<'a> {
    pub aliases: BTreeMap<&'a String, &'a UnresolvedVersionSpec>,
}

#[component]
pub fn AliasesMap<'a>(props: &AliasesMapProps<'a>) -> impl Into<AnyElement<'a>> {
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
                                style: Style::Hash
                            )
                        }.into_any()
                    )
                }
            }))
        }
    }
}
