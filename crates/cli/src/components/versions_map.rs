use super::create_datetime;
use iocraft::prelude::*;
use proto_core::layout::Inventory;
use proto_core::{UnresolvedVersionSpec, VersionSpec};
use starbase_console::ui::{Map, MapItem, Style, StyledText};

#[derive(Default, Props)]
pub struct VersionsMapProps<'a> {
    pub default_version: Option<&'a UnresolvedVersionSpec>,
    pub inventory: Option<&'a Inventory>,
    pub versions: Vec<&'a VersionSpec>,
}

#[component]
pub fn VersionsMap<'a>(props: &VersionsMapProps<'a>) -> impl Into<AnyElement<'a>> + use<'a> {
    let inventory = props.inventory.expect("`inventory` prop is required!");

    element! {
        Map {
            #(props.versions.iter().map(|version| {
                let mut comments = vec![];
                let mut is_default = false;

                if let Some(meta) = inventory.manifest.versions.get(version) {
                    if let Some(at) = create_datetime(meta.installed_at) {
                        comments.push(format!("installed {}", at.format("%x")));
                    }

                    if let Ok(Some(last_used)) = inventory.create_product(version).load_used_at()
                        && let Some(at) = create_datetime(last_used) {
                            comments.push(format!("last used {}", at.format("%x")));
                        }
                }

                if props.default_version.is_some_and(|dv| dv == &version.to_unresolved_spec()) {
                    comments.push("fallback version".into());
                    is_default = true;
                }

                element! {
                    MapItem(
                        name: element! {
                            StyledText(
                                content: version.to_string(),
                                style: if is_default {
                                    Style::Shell
                                } else {
                                    Style::Hash
                                }
                            )
                        }.into_any(),
                        value: element! {
                            StyledText(
                                content: comments.join(", "),
                                style: Style::MutedLight
                            )
                        }.into_any(),
                        separator: "-".to_owned(),
                    )
                }
            }))
        }
    }
}
