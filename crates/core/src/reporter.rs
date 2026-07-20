use iocraft::prelude::element;
use serde::Serialize;
use starbase_console::ui::*;
use starbase_console::{Console, ConsoleError, ConsoleStream, ConsoleStreamType, Reporter};
use starbase_styles::{apply_style_tags, color, remove_style_tags};
use starbase_utils::json::serde_json;
use std::sync::{Arc, RwLock};

pub type ProtoConsole = Console<ProtoReporter>;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum ReporterFormat {
    #[default]
    #[cfg_attr(feature = "clap", value(alias = "user", alias = "human"))]
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
    stderr_only: bool,
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

    pub fn new_stderr(format: ReporterFormat) -> Self {
        Self {
            format,
            stderr_only: true,
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
            stderr_only: false,
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
            self.err = err.clone();
            self.out = if self.stderr_only { err } else { out };
        }
    }
}

impl ProtoReporter {
    pub fn is_json_format(&self) -> bool {
        self.format.is_json()
    }

    pub fn append_json<T: Serialize>(&self, value: T) -> Result<(), ConsoleError> {
        if let Ok(mut buffer) = self.json_buffer.write() {
            let content = serde_json::to_string_pretty(&value).map_err(|error| {
                ConsoleError::WriteJsonFailed {
                    error: Box::new(error),
                }
            })?;

            buffer.push(remove_style_tags(content));
        }

        Ok(())
    }

    pub fn flush_json(&self) -> Result<(), ConsoleError> {
        if let Ok(mut buffer) = self.json_buffer.write() {
            let mut content = buffer.drain(..).collect::<Vec<_>>();

            match content.len() {
                0 => {}
                1 => self.out.write_line(content.remove(0))?,
                _ => self
                    .out
                    .write_line(format!("[\n{}\n]", content.join(",\n")))?,
            };
        }

        Ok(())
    }

    pub fn write_json<T: Serialize>(&self, value: T) -> Result<(), ConsoleError> {
        let content =
            serde_json::to_string(&value).map_err(|error| ConsoleError::WriteJsonFailed {
                error: Box::new(error),
            })?;

        self.out.write_line(remove_style_tags(content))?;

        Ok(())
    }

    pub fn write_json_pretty<T: Serialize>(&self, value: T) -> Result<(), ConsoleError> {
        let content = serde_json::to_string_pretty(&value).map_err(|error| {
            ConsoleError::WriteJsonFailed {
                error: Box::new(error),
            }
        })?;

        self.out.write_line(remove_style_tags(content))?;

        Ok(())
    }

    pub fn write_json_for_format<T: Serialize>(&self, value: T) -> Result<(), ConsoleError> {
        if self.format == ReporterFormat::Ndjson {
            self.write_json(Data::Data(value))
        } else {
            self.write_json_pretty(value)
        }
    }

    pub fn main_error(
        &self,
        message: impl Into<String>,
        code: Option<String>,
    ) -> Result<(), ConsoleError> {
        let message = message.into();

        match self.format {
            ReporterFormat::Text => {
                return self.notice(Variant::Failure, message);
            }
            // Do not use append here! This happens at the end of main!
            ReporterFormat::Json | ReporterFormat::Ndjson => {
                self.write_json(Event::Error(MessageOutput { code, message }))?;
            }
        };

        self.out.flush()?;

        Ok(())
    }

    pub fn message(&self, message: impl Into<String>) -> Result<(), ConsoleError> {
        let message = message.into();

        match self.format {
            ReporterFormat::Text => {
                self.out.write_line(apply_styles(message))?;
            }
            ReporterFormat::Json => {
                self.append_json(message)?;
            }
            ReporterFormat::Ndjson => {
                self.write_json(Event::Message(MessageOutput {
                    code: None,
                    message,
                }))?;
            }
        };

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

    pub fn notice_with(&self, output: NoticeOutput) -> Result<(), ConsoleError> {
        match self.format {
            ReporterFormat::Text => {
                let el = element! {
                    View {
                        Notice(title: output.title, variant: output.variant) {
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

    pub fn progress(
        &self,
        message: impl Into<String>,
        id: Option<String>,
    ) -> Result<(), ConsoleError> {
        let message = message.into();

        match self.format {
            ReporterFormat::Text => match id {
                Some(id) => self.out.write_line_with_prefix(
                    apply_styles(message),
                    &color::muted_light(format!("[{id}] ")),
                )?,
                None => self.out.write_line(apply_styles(message))?,
            },
            ReporterFormat::Json => {
                self.append_json(ProgressOutput { id, message })?;
            }
            ReporterFormat::Ndjson => {
                self.write_json(Event::Progress(ProgressOutput { id, message }))?;
            }
        };

        Ok(())
    }

    pub fn table(
        &self,
        headers: Vec<TableHeader>,
        cells: Vec<Vec<String>>,
    ) -> Result<(), ConsoleError> {
        self.table_with(TableOutput {
            headers_config: headers,
            cells,
            ..Default::default()
        })
    }

    pub fn table_with(&self, mut output: TableOutput) -> Result<(), ConsoleError> {
        output.headers = output
            .headers_config
            .iter()
            .map(|header| header.label.clone())
            .collect();

        match self.format {
            ReporterFormat::Text => {
                let el = element! {
                    Container {
                        Table(
                            headers: output.headers_config,
                        ) {
                            #(output.cells.into_iter().enumerate().map(|(row_index, row)| element! {
                                TableRow(row: row_index as i32) {
                                    #(row.into_iter().enumerate().map(|(col_index, cell)| element! {
                                        TableCol(col: col_index as i32) {
                                            StyledText(content: cell)
                                        }
                                    }))
                                }
                            }))
                        }
                    }
                };

                self.out.render(el, self.theme.clone())?;
            }
            ReporterFormat::Json => {
                self.append_json(output)?;
            }
            ReporterFormat::Ndjson => {
                self.write_json(Event::Table(output))?;
            }
        };

        Ok(())
    }
}

fn apply_styles(message: String) -> String {
    apply_style_tags(
        // Compatibility with the UI theme
        message
            .replace("version>", "hash>")
            .replace("versionalt>", "symbol>"),
    )
}

#[derive(Default, Serialize)]
pub struct MessageOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
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

#[derive(Default, Serialize)]
pub struct ProgressOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub message: String,
}

#[derive(Default, Serialize)]
pub struct TableOutput {
    #[serde(skip)]
    pub headers_config: Vec<TableHeader>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<Vec<String>>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Error(MessageOutput),
    Message(MessageOutput),
    Notice(NoticeOutput),
    Progress(ProgressOutput),
    Table(TableOutput),
}

#[derive(Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Data<T: Serialize> {
    Data(T),
}
