// raw line
// custom json
// custom jsx
// table data

use iocraft::prelude::element;
use serde::Serialize;
use starbase_console::ui::*;
use starbase_console::{Console, ConsoleError, ConsoleStream, ConsoleStreamType, Reporter};
use starbase_styles::remove_style_tags;
use starbase_utils::json::serde_json;
use std::sync::{Arc, RwLock};

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
    Ndjson,
}

impl ReporterFormat {
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json | Self::Ndjson)
    }
}

#[derive(Debug)]
pub struct ProtoReporter {
    format: ReporterFormat,
    err: ConsoleStream,
    out: ConsoleStream,
    theme: ConsoleTheme,
    test_mode: bool,
    json_buffer: Arc<RwLock<Vec<String>>>,
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
            err: ConsoleStream::new_testing(ConsoleStreamType::Stderr),
            out: ConsoleStream::new_testing(ConsoleStreamType::Stdout),
            test_mode: true,
            ..Default::default()
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
            json_buffer: Arc::new(RwLock::new(Vec::new())),
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
    fn append_json<T: Serialize>(&self, value: T) -> Result<(), ConsoleError> {
        if let Ok(mut buffer) = self.json_buffer.write() {
            buffer.push(serde_json::to_string_pretty(&value).map_err(|error| {
                ConsoleError::WriteJsonFailed {
                    error: Box::new(error),
                }
            })?);
        }

        Ok(())
    }

    fn write_json<T: Serialize>(&self, value: T) -> Result<(), ConsoleError> {
        let content =
            serde_json::to_string(&value).map_err(|error| ConsoleError::WriteJsonFailed {
                error: Box::new(error),
            })?;

        self.out.write_line(content)?;

        Ok(())
    }

    pub fn notice(&self, variant: Variant, message: impl Into<String>) -> Result<(), ConsoleError> {
        self.notice_with(NoticeOutput {
            variant,
            title: None,
            messages: vec![message.into()],
            items: vec![],
        })
    }

    pub fn notice_with(&self, mut output: NoticeOutput) -> Result<(), ConsoleError> {
        if self.format.is_json() {
            output.messages = remove_tags(output.messages);
            output.items = remove_tags(output.items);
        }

        match self.format {
            ReporterFormat::Text => {
                let el = element! {
                    View {
                        Notice(variant: output.variant) {
                            #(output.messages.into_iter().map(|message| element! {
                                StyledText(content: message)
                            }))
                        }
                        #(if output.items.is_empty() {
                            None
                        } else {
                            Some(element! {
                                List {
                                    #(output.items.into_iter().map(|item| element! {
                                        ListItem {
                                            StyledText(content: item)
                                        }
                                    }))
                                }
                            })
                        })
                    }
                };

                if matches!(output.variant, Variant::Caution | Variant::Failure) {
                    self.err.render(el, self.theme.clone())?;
                } else {
                    self.out.render(el, self.theme.clone())?;
                }
            }
            ReporterFormat::Json => {
                self.append_json(output)?;
            }
            ReporterFormat::Ndjson => {
                self.write_json(Event::Notice(output))?;
            }
        };

        Ok(())
    }
}

fn remove_tags(values: Vec<String>) -> Vec<String> {
    values.into_iter().map(remove_style_tags).collect()
}

#[derive(Default, Serialize)]
pub struct NoticeOutput {
    pub variant: Variant,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Notice(NoticeOutput),
}
