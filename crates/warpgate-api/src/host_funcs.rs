use crate::virtual_path::VirtualPath;
use crate::{AnyResult, api_struct, api_unit_enum};
use derive_setters::Setters;
use rustc_hash::FxHashMap;
use serde::de::DeserializeOwned;

api_unit_enum!(
    /// Target where host logs should be written to.
    pub enum HostLogTarget {
        // Console
        Stderr,
        Stdout,
        // Levels
        Debug,
        Error,
        Trace,
        Warn,
        #[default]
        Tracing,
    }
);

api_struct!(
    /// Input passed to the `host_log` host function.
    #[derive(Setters)]
    #[serde(default)]
    pub struct HostLogInput {
        pub data: FxHashMap<String, serde_json::Value>,

        #[setters(into)]
        pub message: String,

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
    #[derive(Setters)]
    #[serde(default)]
    pub struct ExecCommandInput {
        /// The command or script to execute. Accepts an executable
        /// on `PATH` or a virtual path.
        #[setters(into)]
        pub command: String,

        /// Arguments to pass to the command.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub args: Vec<String>,

        /// Environment variables to pass to the command.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub env: FxHashMap<String, String>,

        /// Mark the command as executable before executing.
        #[setters(skip)]
        #[doc(hidden)]
        pub set_executable: bool,

        /// Set the shell to execute the command with, for example "bash".
        /// If not defined, will be detected from the parent process.
        #[setters(into, strip_option)]
        pub shell: Option<String>,

        /// Stream the output instead of capturing it.
        #[setters(bool)]
        pub stream: bool,

        /// Override the current working directory.
        #[setters(strip_option)]
        #[serde(alias = "working_dir", skip_serializing_if = "Option::is_none")]
        pub cwd: Option<VirtualPath>,
    }
);

impl ExecCommandInput {
    /// Create a new command that inherits and streams the output.
    pub fn new<C, I, V>(command: C, args: I) -> ExecCommandInput
    where
        C: AsRef<str>,
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        let mut input = Self::pipe(command, args);
        input.stream = true;
        input
    }

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
        Self::new(command, args)
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
    #[derive(Setters)]
    pub struct SendRequestInput {
        /// The URL to send to.
        #[setters(into)]
        pub url: String,

        /// HTTP headers to inject into the request.
        #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
        pub headers: FxHashMap<String, String>,
    }
);

impl SendRequestInput {
    /// Create a new send request with the provided url.
    pub fn new(url: impl AsRef<str>) -> Self {
        Self {
            url: url.as_ref().to_owned(),
            ..Default::default()
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
