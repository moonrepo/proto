use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input passed to the `trace` host function.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TraceInput {
    Message(String),
    Fields {
        data: HashMap<String, serde_json::Value>,
        message: String,
    },
}

impl From<&str> for TraceInput {
    fn from(message: &str) -> Self {
        TraceInput::Message(message.to_owned())
    }
}

impl From<String> for TraceInput {
    fn from(message: String) -> Self {
        TraceInput::Message(message)
    }
}

/// Input passed to the `exec_command` host function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ExecCommandInput {
    /// Arguments to pass to the command.
    pub args: Vec<String>,

    /// The command to execute.
    pub command: String,

    /// Environment variables to pass to the command.
    pub env_vars: HashMap<String, String>,
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
        }
    }
}

/// Output returned from the `exec_command` host function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ExecCommandOutput {
    pub exit_code: i32,
    pub stderr: String,
    pub stdout: String,
}
