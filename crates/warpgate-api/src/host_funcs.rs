use crate::virtual_path::VirtualPath;
use crate::{api_enum, api_struct};
use rustc_hash::FxHashMap;

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
    #[serde(default)]
    pub struct HostLogInput {
        pub data: FxHashMap<String, serde_json::Value>,

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
    #[serde(default)]
    pub struct ExecCommandInput {
        /// Arguments to pass to the command.
        pub args: Vec<String>,

        /// The command or script to execute.
        pub command: String,

        /// Environment variables to pass to the command.
        pub env: FxHashMap<String, String>,

        /// Mark the command as executable before executing.
        #[doc(hidden)]
        pub set_executable: bool,

        /// Stream the output instead of capturing it.
        pub stream: bool,

        /// Override the current working directory.
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
