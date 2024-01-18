/// Wrap a struct with common derives and serde required attributes.
#[macro_export]
macro_rules! api_struct {
    ($struct:item) => {
        #[derive(Clone, Debug, Default, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
        #[serde(default)]
        $struct
    };
}

/// Wrap an enum with common derives and serde required attributes.
#[macro_export]
macro_rules! api_enum {
    ($struct:item) => {
        #[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
        $struct
    };
}

/// Calls the `exec_command` host function to execute a command on
/// the host as a synchronous child process.
#[macro_export]
macro_rules! exec_command {
    (raw, $cmd:literal) => {
        exec_command!(raw, ExecCommandInput {
            command: $cmd.into(),
            ..ExecCommandInput::default()
        })
    };
    (raw, $cmd:expr, $args:expr) => {
        exec_command!(raw, ExecCommandInput::pipe($cmd, $args))
    };
    (raw, $input:expr) => {
        unsafe { exec_command(Json($input)) }
    };
    (pipe, $cmd:expr, $args:expr) => {
        exec_command!(ExecCommandInput::pipe($cmd, $args))
    };
    (inherit, $cmd:expr, $args:expr) => {
        exec_command!(ExecCommandInput::inherit($cmd, $args))
    };
    ($cmd:expr, [ $($arg:literal),* ]) => {
        exec_command!(pipe, $cmd, [ $($arg),* ])
    };
    ($input:expr) => {
        unsafe { exec_command(Json($input))?.0 }
    };
}

/// Calls the `get_env_var` or `set_env_var` host function to manage
/// environment variables on the host.
///
/// When setting `PATH`, the provided value will append to `PATH`,
/// not overwrite it. Supports both `;` and `:` delimiters.
#[macro_export]
macro_rules! host_env {
    ($name:literal, $value:expr) => {
        unsafe { set_env_var($name.into(), $value.into())? };
    };
    ($name:literal) => {
        unsafe {
            let inner = get_env_var($name.into())?;

            if inner.is_empty() {
                None
            } else {
                Some(inner)
            }
        }
    };
}

/// Calls the `host_log` host function to log a message to the host's terminal.
#[macro_export]
macro_rules! host_log {
    (stdout, $($arg:tt)+) => {
        unsafe {
            host_log(Json(HostLogInput {
                message: format!($($arg)+),
                target: HostLogTarget::Stdout,
                ..HostLogInput::default()
            }))?;
        }
    };
    (stdout, $msg:literal) => {
        unsafe {
            host_log(Json(HostLogInput {
                message: $msg.into(),
                target: HostLogTarget::Stdout,
                ..HostLogInput::default()
            }))?;
        }
    };
    (stderr, $($arg:tt)+) => {
        unsafe {
            host_log(Json(HostLogInput {
                message: format!($($arg)+),
                target: HostLogTarget::Stderr,
                ..HostLogInput::default()
            }))?;
        }
    };
    (stderr, $msg:literal) => {
        unsafe {
            host_log(Json(HostLogInput {
                message: $msg.into(),
                target: HostLogTarget::Stderr,
                ..HostLogInput::default()
            }))?;
        }
    };
    ($($arg:tt)+) => {
        unsafe {
            host_log(Json(format!($($arg)+).into()))?;
        }
    };
    ($msg:literal) => {
        unsafe {
            host_log(Json($msg.into()))?;
        }
    };
    ($input:expr) => {
        unsafe {
            host_log(Json($input))?;
        }
    };
}

/// Calls `from_virtual_path` on the host to convert the provided value to a real path
/// from a virtual path.
#[macro_export]
macro_rules! real_path {
    (buf, $path:expr) => {
        real_path!($path.to_string_lossy())
    };
    ($path:literal) => {
        std::path::PathBuf::from(unsafe { from_virtual_path($path.to_owned())? })
    };
    ($path:expr) => {
        std::path::PathBuf::from(unsafe { from_virtual_path($path.try_into()?)? })
    };
}

/// Calls `to_virtual_path` on the host to convert the provided value to a virtual path
/// from a real path.
#[macro_export]
macro_rules! virtual_path {
    (buf, $path:expr) => {
        virtual_path!($path.to_string_lossy())
    };
    ($path:literal) => {
        std::path::PathBuf::from(unsafe { to_virtual_path($path.to_owned())? })
    };
    ($path:expr) => {
        std::path::PathBuf::from(unsafe { to_virtual_path($path.try_into()?)? })
    };
}
