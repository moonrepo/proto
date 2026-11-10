use crate::virtual_path::VirtualPath;
use crate::{AnyResult, api_struct, api_unit_enum};
use derive_setters::Setters;
use rustc_hash::FxHashMap;
use serde::de::DeserializeOwned;
use std::path::PathBuf;

api_unit_enum!(
    /// Target where host logs should be written to.
    pub enum HostLogTarget {
        /// Write to the standard error console stream.
        Stderr,

        /// Write to the standard output console stream.
        Stdout,

        /// Log a message with the error level.
        Error,

        /// Log a message with the warn level.
        Warn,

        /// Log a message with the debug level.
        Debug,

        /// Log a message with the trace level.
        #[default]
        Trace,
    }
);

api_struct!(
    /// Input passed to the `host_log` host function.
    #[derive(Setters)]
    #[serde(default)]
    pub struct HostLogInput {
        /// Additional data/fields to log.
        pub data: FxHashMap<String, serde_json::Value>,

        /// The message to log.
        #[setters(into)]
        pub message: String,

        /// Target where the log should be written to.
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
        /// name available on `PATH` or a virtual path.
        #[setters(into)]
        pub command: String,

        /// Arguments to pass to the command.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub args: Vec<String>,

        /// Override the current working directory.
        #[setters(strip_option)]
        #[serde(alias = "working_dir", skip_serializing_if = "Option::is_none")]
        pub cwd: Option<VirtualPath>,

        /// Environment variables to pass to the command. Variables
        /// can customize behavior by appending one of the following
        /// characters to the name:
        ///
        ///  `?` - Will only set variable if it doesn't exist
        ///        in the current environment.
        ///  `!` - Will remove the variable from being inherited
        ///        by the child process.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub env: FxHashMap<String, String>,

        /// List of real or virtual paths to prepend to the `PATH`
        /// environment variable when executing the command.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub paths: Vec<PathBuf>,

        /// Mark the command as executable before executing.
        #[setters(skip)]
        #[doc(hidden)]
        pub set_executable: bool,

        /// Set the shell to execute the command with, for example "bash".
        #[setters(into, strip_option)]
        pub shell: Option<String>,

        /// Stream the output instead of capturing it.
        #[setters(bool)]
        pub stream: bool,
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
            args: args
                .into_iter()
                .map(|arg| arg.as_ref().to_owned())
                .collect(),
            ..Default::default()
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
        /// The command (without arguments) that was executed.
        pub command: String,

        /// The exit code returned from the command.
        pub exit_code: i32,

        /// The standard error output returned from the command.
        pub stderr: String,

        /// The standard output returned from the command.
        pub stdout: String,

        /// Whether the command was streamed (inherit) or piped.
        pub streamed: bool,
    }
);

impl ExecCommandOutput {
    /// Get the combined output of stdout and stderr, trimmed of whitespace.
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

api_unit_enum!(
    /// HTTP method to send a request with.
    pub enum SendRequestMethod {
        /// Send a `GET` request.
        #[default]
        #[serde(alias = "GET")]
        Get,

        /// Send a `POST` request.
        #[serde(alias = "POST")]
        Post,
    }
);

api_struct!(
    /// Input passed to the `download_file` host function.
    #[derive(Setters)]
    pub struct DownloadFileInput {
        /// The URL to download a file from.
        #[setters(into)]
        pub url: String,

        /// Virtual path of the destination file to write the
        /// downloaded contents to.
        pub file: VirtualPath,

        /// HTTP headers to inject into the request.
        #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
        pub headers: FxHashMap<String, String>,
    }
);

impl DownloadFileInput {
    /// Create a new download request with the provided URL and destination file.
    pub fn new(url: impl AsRef<str>, file: impl AsRef<std::ffi::OsStr>) -> Self {
        Self {
            url: url.as_ref().to_owned(),
            file: VirtualPath::new(file),
            headers: FxHashMap::default(),
        }
    }
}

api_struct!(
    /// Output returned from the `download_file` host function.
    #[serde(default)]
    pub struct DownloadFileOutput {
        /// Virtual path of the destination file that was written to.
        pub file: VirtualPath,

        /// The size of the downloaded file, in bytes.
        pub size: u64,
    }
);

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

        /// HTTP method to send the request with.
        #[serde(default)]
        pub method: SendRequestMethod,
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

    /// Create a new send request with the provided url,
    /// that sends with the `POST` method.
    pub fn post(url: impl AsRef<str>) -> Self {
        Self {
            method: SendRequestMethod::Post,
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
        /// The response body as raw bytes. When empty, the body must be
        /// loaded from WASM memory using the offset and length.
        pub body: Vec<u8>,

        /// Length of the response body stored in WASM memory.
        pub body_length: u64,

        /// Offset of the response body stored in WASM memory.
        pub body_offset: u64,

        /// The response status code.
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
