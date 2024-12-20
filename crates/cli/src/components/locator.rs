use iocraft::prelude::*;
use proto_core::PluginLocator;
use starbase_console::ui::{Entry, Style, StyledText};

#[derive(Default, Props)]
pub struct LocatorProps<'a> {
    pub value: Option<&'a PluginLocator>,
}

#[component]
pub fn Locator<'a>(props: &LocatorProps<'a>) -> impl Into<AnyElement<'a>> {
    match props.value.as_ref().expect("`value` prop is required!") {
        PluginLocator::File(file) => element! {
            Entry(
                name: "Source file",
                value: element! {
                    StyledText(
                        content: file.get_resolved_path().to_string_lossy(),
                        style: Style::Path
                    )
                }.into_any()
            )
        }
        .into_any(),
        PluginLocator::GitHub(github) => element! {
            Entry(
                name: "Source",
                content: "GitHub".to_owned()
            ) {
                Box(flex_direction: FlexDirection::Column) {
                    Entry(
                        name: "Repository",
                        value: element! {
                            StyledText(
                                content: &github.repo_slug,
                                style: Style::Id
                            )
                        }.into_any()
                    )
                    #(github.project_name.as_ref().map(|name| {
                        element! {
                            Entry(
                                name: "Project",
                                value: element! {
                                    StyledText(
                                        content: name,
                                        style: Style::Label
                                    )
                                }.into_any()
                            )
                        }
                    }))
                    Entry(
                        name: "Tag",
                        value: element! {
                            StyledText(
                                content: github.tag.as_deref().unwrap_or("latest"),
                                style: Style::Hash
                            )
                        }.into_any()
                    )
                }
            }
        }
        .into_any(),
        PluginLocator::Url(url) => element! {
            Entry(
                name: "Source URL",
                value: element! {
                    StyledText(
                        content: &url.url,
                        style: Style::Url
                    )
                }.into_any()
            )
        }
        .into_any(),
    }
}
