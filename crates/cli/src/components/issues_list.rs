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
pub fn IssuesList<'a>(props: &IssuesListProps) -> impl Into<AnyElement<'a>> + use<'a> {
    element! {
        List(gap: 1) {
            #(props.issues.iter().map(|issue| {
                element! {
                    ListItem {
                        View {
                            StyledText(content: "Issue: ", style: Style::MutedLight)
                            StyledText(content: &issue.issue)
                        }
                        #(issue.resolution.as_ref().map(|resolution| {
                            element! {
                                View {
                                    StyledText(content: "Resolution: ", style: Style::MutedLight)
                                    StyledText(content: resolution)
                                }
                            }
                        }))
                        #(issue.comment.as_ref().map(|comment| {
                            element! {
                                View {
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
