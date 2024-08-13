use crate::virtual_path::VirtualPath;
use crate::{api_enum, api_struct, AnyResult};
use rustc_hash::FxHashMap;
use serde::de::DeserializeOwned;

api_enum!(
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

api_struct!(
    /// Input passed to the `host_log` host function.
    pub struct HostLogInput {
        #[serde(default)]
        pub data: FxHashMap<String, serde_json::Value>,

        pub message: String,

        #[serde(default)]
        pub target: HostLogTarget,
    }
);

impl HostLogInput {
    /// Create a new host log with the provided message.
    pub fn new(message: impl AsRef<str>) -> Self {
        Self {
            message: message.as_ref().to_owned(),
            ..Default::default()
        }
    }
}

impl From<&str> for HostLogInput {
    fn from(message: &str) -> Self {
        HostLogInput::new(message)
    }
}

impl From<String> for HostLogInput {
    fn from(message: String) -> Self {
        HostLogInput::new(message)
    }
}

api_struct!(
    /// Input passed to the `exec_command` host function.
    pub struct ExecCommandInput {
        /// Arguments to pass to the command.
        pub args: Vec<String>,

        /// The command or script to execute.
        pub command: String,

        /// Environment variables to pass to the command.
        #[serde(default)]
        pub env: FxHashMap<String, String>,

        /// Mark the command as executable before executing.
        #[doc(hidden)]
        pub set_executable: bool,

        /// Stream the output instead of capturing it.
        #[serde(default)]
        pub stream: bool,

        /// Override the current working directory.
        #[serde(default)]
        pub working_dir: Option<VirtualPath>,
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

api_struct!(
    /// Output returned from the `exec_command` host function.
    #[serde(default)]
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

api_struct!(
    /// Input passed to the `send_request` host function.
    pub struct SendRequestInput {
        /// The URL to send to.
        pub url: String,
    }
);

impl SendRequestInput {
    /// Create a new send request with the provided url.
    pub fn new(url: impl AsRef<str>) -> Self {
        Self {
            url: url.as_ref().to_owned(),
        }
    }
}

impl From<&str> for SendRequestInput {
    fn from(url: &str) -> Self {
        SendRequestInput::new(url)
    }
}

impl From<String> for SendRequestInput {
    fn from(url: String) -> Self {
        SendRequestInput::new(url)
    }
}

api_struct!(
    /// Output returned from the `send_request` host function.
    pub struct SendRequestOutput {
        pub body: Vec<u8>,
        pub body_length: u64,
        pub body_offset: u64,
        pub status: u16,
    }
);

impl SendRequestOutput {
    /// Consume the response body and return as JSON.
    pub fn json<T: DeserializeOwned>(self) -> AnyResult<T> {
        Ok(serde_json::from_slice(&self.body)?)
    }

    /// Consume the response body and return as raw text.
    pub fn text(self) -> AnyResult<String> {
        Ok(String::from_utf8(self.body)?)
    }
}
