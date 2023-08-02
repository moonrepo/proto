use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input passed to the `trace` host function.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum HostLogInput {
    Message(String),
    Fields {
        data: HashMap<String, serde_json::Value>,
        message: String,
    },
}

impl From<&str> for HostLogInput {
    fn from(message: &str) -> Self {
        HostLogInput::Message(message.to_owned())
    }
}

impl From<String> for HostLogInput {
    fn from(message: String) -> Self {
        HostLogInput::Message(message)
    }
}

/// Input passed to the `exec_command` host function.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExecCommandInput {
    /// Arguments to pass to the command.
    pub args: Vec<String>,

    /// The command to execute.
    pub command: String,

    /// Environment variables to pass to the command.
    pub env_vars: HashMap<String, String>,

    /// Stream the output instead of capturing it.
    pub stream: bool,
}

impl ExecCommandInput {
    pub fn new<I, V>(command: &str, args: I) -> ExecCommandInput
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        ExecCommandInput {
            command: command.to_string(),
            args: args.into_iter().map(|a| a.as_ref().to_owned()).collect(),
            env_vars: HashMap::new(),
            stream: false,
        }
    }

    pub fn stream<I, V>(command: &str, args: I) -> ExecCommandInput
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        let mut input = Self::new(command, args);
        input.stream = true;
        input
    }
}

/// Output returned from the `exec_command` host function.
#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExecCommandOutput {
    pub exit_code: i32,
    pub stderr: String,
    pub stdout: String,
}
