/// Generate all permutations for the provided OS and architecture mapping.
#[macro_export]
macro_rules! permutations {
  [ $( $os:path => [ $($arch:path),* ], )* ] => {
    std::collections::HashMap::from_iter([
      $(
        (
          $os,
          Vec::from_iter([
            $(
              $arch
            ),*
          ])
        ),
      )*
    ])
  };
}

/// Return a [`PluginError`] wrapped in [`WithReturnCode`].
#[macro_export]
macro_rules! err {
    ($msg:literal) => {
        Err(WithReturnCode::new(
            PluginError::Message($msg.into()).into(),
            1,
        ))
    };
    ($msg:literal, $($arg:tt)*) => {
        Err(WithReturnCode::new(
            PluginError::Message(format!($msg, $($arg)*)).into(),
            1,
        ))
    };
    ($msg:expr) => {
        Err(WithReturnCode::new($msg, 1))
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
#[macro_export]
macro_rules! host_env {
    ($name:literal, $value:expr) => {
        unsafe { set_env_var($name, $value)? };
    };
    ($name:literal) => {
        unsafe {
            let inner = get_env_var($name)?;

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
            host_log(Json(HostLogInput::TargetedMessage {
                message: format!($($arg)+),
                target: HostLogTarget::Stdout,
            }))?;
        }
    };
    (stdout, $msg:literal) => {
        unsafe {
            host_log(Json(HostLogInput::TargetedMessage {
                message: $msg.into(),
                target: HostLogTarget::Stdout,
            }))?;
        }
    };
    (stderr, $($arg:tt)+) => {
        unsafe {
            host_log(Json(HostLogInput::TargetedMessage {
                message: format!($($arg)+),
                target: HostLogTarget::Stderr,
            }))?;
        }
    };
    (stderr, $msg:literal) => {
        unsafe {
            host_log(Json(HostLogInput::TargetedMessage {
                message: $msg.into(),
                target: HostLogTarget::Stderr,
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

/// Calls `from_virtual_path` on the host to convert the provided value from a real path.
#[macro_export]
macro_rules! real_path {
    ($path:literal) => {
        std::path::PathBuf::from(unsafe { from_virtual_path($path)? })
    };
    ($path:expr) => {
        std::path::PathBuf::from(unsafe { from_virtual_path($path.to_str().unwrap())? })
    };
}

/// Calls `to_virtual_path` on the host to convert the provided value from a virtual path.
#[macro_export]
macro_rules! virtual_path {
    ($path:literal) => {
        std::path::PathBuf::from(unsafe { to_virtual_path($path)? })
    };
    ($path:expr) => {
        std::path::PathBuf::from(unsafe { to_virtual_path($path.to_str().unwrap())? })
    };
}
