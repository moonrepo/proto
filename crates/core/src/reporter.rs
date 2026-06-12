// raw line
// custom json
// custom jsx
// table data

use crate::helpers::now;
use iocraft::prelude::element;
use serde::Serialize;
use starbase_console::ui::*;
use starbase_console::{Console, ConsoleError, ConsoleStream, ConsoleStreamType, Reporter};
use starbase_styles::remove_style_tags;
use starbase_utils::json::serde_json;

pub type ProtoConsole = Console<ProtoReporter>;

#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum ReporterFormat {
    #[default]
    #[cfg_attr(feature = "clap", value(alias = "user"))]
    Text,
    #[cfg_attr(feature = "clap", value(alias = "data"))]
    Json,
    #[cfg_attr(feature = "clap", value(alias = "bot", alias = "agent"))]
    NdJson,
}

impl ReporterFormat {
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json | Self::NdJson)
    }
}

#[derive(Debug)]
pub struct ProtoReporter {
    format: ReporterFormat,
    err: ConsoleStream,
    out: ConsoleStream,
    theme: ConsoleTheme,
    test_mode: bool,
}

impl ProtoReporter {
    pub fn new(format: ReporterFormat) -> Self {
        Self {
            format,
            ..Default::default()
        }
    }

    pub fn new_testing() -> Self {
        Self {
            format: ReporterFormat::Text,
            err: ConsoleStream::new_testing(ConsoleStreamType::Stderr),
            out: ConsoleStream::new_testing(ConsoleStreamType::Stdout),
            theme: ConsoleTheme::default(),
            test_mode: true,
        }
    }
}

impl Default for ProtoReporter {
    fn default() -> Self {
        Self {
            format: ReporterFormat::Text,
            err: ConsoleStream::empty(ConsoleStreamType::Stderr),
            out: ConsoleStream::empty(ConsoleStreamType::Stdout),
            theme: ConsoleTheme::default(),
            test_mode: false,
        }
    }
}

impl Reporter for ProtoReporter {
    fn inherit_theme(&mut self, theme: ConsoleTheme) {
        self.theme = theme;
    }

    fn inherit_streams(&mut self, err: ConsoleStream, out: ConsoleStream) {
        if !self.test_mode {
            self.err = err;
            self.out = out;
        }
    }
}

impl ProtoReporter {
    pub fn write_json<T: Serialize>(&self, value: T) -> Result<(), ConsoleError> {
        let content =
            serde_json::to_string(&value).map_err(|error| ConsoleError::WriteJsonFailed {
                error: Box::new(error),
            })?;

        self.out.write_line(content)?;

        Ok(())
    }

    pub fn write_json_pretty<T: Serialize>(&self, value: T) -> Result<(), ConsoleError> {
        let content = serde_json::to_string_pretty(&value).map_err(|error| {
            ConsoleError::WriteJsonFailed {
                error: Box::new(error),
            }
        })?;

        self.out.write_line(content)?;

        Ok(())
    }

    pub fn notice(&self, variant: Variant, messages: Vec<String>) -> Result<(), ConsoleError> {
        self.notice_with_items(variant, messages, vec![])
    }

    pub fn notice_with_items(
        &self,
        variant: Variant,
        messages: Vec<String>,
        items: Vec<String>,
    ) -> Result<(), ConsoleError> {
        match self.format {
            ReporterFormat::Text => {
                let el = element! {
                    View {
                        Notice(variant) {
                            #(messages.into_iter().map(|message| element! {
                                StyledText(content: message)
                            }))
                        }
                        #(if items.is_empty() {
                            None
                        } else {
                            Some(element! {
                                List {
                                    #(items.into_iter().map(|item| element! {
                                        ListItem {
                                            StyledText(content: item)
                                        }
                                    }))
                                }
                            })
                        })
                    }
                };

                if matches!(variant, Variant::Caution | Variant::Failure) {
                    self.err.render(el, self.theme.clone())?;
                } else {
                    self.out.render(el, self.theme.clone())?;
                }
            }
            ReporterFormat::Json => {
                self.write_json_pretty(NoticeOutput {
                    variant,
                    messages: remove_tags(messages),
                    items: remove_tags(items),
                })?;
            }
            ReporterFormat::NdJson => {
                self.write_json(Event::Notice {
                    variant,
                    messages: remove_tags(messages),
                    items: remove_tags(items),
                    timestamp: now(),
                })?;
            }
        };

        Ok(())
    }
}

fn remove_tags(values: Vec<String>) -> Vec<String> {
    values.into_iter().map(remove_style_tags).collect()
}

#[derive(Serialize)]
pub struct NoticeOutput {
    variant: Variant,
    messages: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    items: Vec<String>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Notice {
        variant: Variant,
        messages: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        items: Vec<String>,
        timestamp: u128,
    },
}
