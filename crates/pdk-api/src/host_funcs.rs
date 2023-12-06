use crate::{json_enum, json_struct};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

json_enum!(
    /// Target where host logs should be written to.
    #[derive(Default)]
    #[serde(rename_all = "lowercase")]
    pub enum HostLogTarget {
        Stderr,
        Stdout,
        #[default]
        Tracing,
    }
);

json_enum!(
    /// Input passed to the `host_log` host function.
    #[serde(untagged)]
    pub enum HostLogInput {
        Message(String),
        TargetedMessage {
            message: String,
            target: HostLogTarget,
        },
        Fields {
            data: HashMap<String, serde_json::Value>,
            message: String,
        },
    }
);

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

json_struct!(
    /// Input passed to the `exec_command` host function.
    pub struct ExecCommandInput {
        /// Arguments to pass to the command.
        pub args: Vec<String>,

        /// The command to execute.
        pub command: String,

        /// Environment variables to pass to the command.
        pub env_vars: HashMap<String, String>,

        /// Mark the command as executable before executing.
        #[doc(hidden)]
        pub set_executable: bool,

        /// Stream the output instead of capturing it.
        pub stream: bool,
    }
);

impl ExecCommandInput {
    /// Create a new command that pipes and captures the output.
    pub fn pipe<C, I, V>(command: C, args: I) -> ExecCommandInput
    where
        C: AsRef<str>,
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        ExecCommandInput {
            command: command.as_ref().to_string(),
            args: args.into_iter().map(|a| a.as_ref().to_owned()).collect(),
            ..ExecCommandInput::default()
        }
    }

    /// Create a new command that inherits and streams the output.
    pub fn inherit<C, I, V>(command: C, args: I) -> ExecCommandInput
    where
        C: AsRef<str>,
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        let mut input = Self::pipe(command, args);
        input.stream = true;
        input
    }
}

json_struct!(
    /// Output returned from the `exec_command` host function.
    pub struct ExecCommandOutput {
        pub command: String,
        pub exit_code: i32,
        pub stderr: String,
        pub stdout: String,
    }
);

impl ExecCommandOutput {
    pub fn get_output(&self) -> String {
        let mut out = String::new();
        out.push_str(self.stdout.trim());

        if !self.stderr.is_empty() {
            if !out.is_empty() {
                out.push(' ');
            }

            out.push_str(self.stderr.trim());
        }

        out
    }
}
