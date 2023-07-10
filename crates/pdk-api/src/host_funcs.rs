use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
