use iocraft::prelude::*;
use serde::Serialize;
use starbase_console::ui::{List, ListItem, Style, StyledText};

#[derive(Debug, Serialize)]
pub struct Issue {
    pub issue: String,
    pub resolution: Option<String>,
    pub comment: Option<String>,
}

#[derive(Default, Props)]
pub struct IssuesListProps {
    pub issues: Vec<Issue>,
}

#[component]
pub fn IssuesList<'a>(props: &IssuesListProps) -> impl Into<AnyElement<'a>> {
    element! {
        List(gap: 1) {
            #(props.issues.iter().map(|issue| {
                element! {
                    ListItem {
                        Box {
                            StyledText(content: "Issue: ", style: Style::MutedLight)
                            StyledText(content: &issue.issue)
                        }
                        #(issue.resolution.as_ref().map(|resolution| {
                            element! {
                                Box {
                                    StyledText(content: "Resolution: ", style: Style::MutedLight)
                                    StyledText(content: resolution)
                                }
                            }
                        }))
                        #(issue.comment.as_ref().map(|comment| {
                            element! {
                                Box {
                                    StyledText(content: "Comment: ", style: Style::MutedLight)
                                    StyledText(content: comment)
                                }
                            }
                        }))
                    }
                }
            }))
        }
    }
}
