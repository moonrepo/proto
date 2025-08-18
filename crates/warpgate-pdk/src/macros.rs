/// Return an error message wrapped in `WithReturnCode` , for use within `#[plugin_fn]`.
#[macro_export]
macro_rules! plugin_err {
    (code = $code:expr, $($arg:tt)+) => {
        WithReturnCode::<Error>::new(anyhow!($($arg)+), $code.into())
    };
    ($($arg:tt)+) => {
        WithReturnCode::<Error>::new(anyhow!($($arg)+), 1)
    };
}

/// Calls the `exec_command` host function to execute a command on
/// the host as a synchronous child process.
#[macro_export]
macro_rules! exec_command {
    (input, $input:expr) => {
        {
            #[allow(clippy::macro_metavars_in_unsafe)]
            unsafe { exec_command(Json($input))?.0 }
        }
    };

    // Raw result
    (raw, $cmd:literal) => {
        exec_command!(raw, $cmd, Vec::<String>::new())
    };
    (raw, $cmd:expr, $args:expr) => {
        exec_command!(raw, ExecCommandInput::pipe($cmd, $args))
    };
    (raw, $input:expr) => {
        {
            #[allow(clippy::macro_metavars_in_unsafe)]
            unsafe { exec_command(Json($input)) }
        }
    };

    // Pipe
    (pipe, $cmd:literal) => {
        exec_command!(pipe, $cmd, Vec::<String>::new())
    };
    (pipe, $cmd:expr, $args:expr) => {
        exec_command!(input, ExecCommandInput::pipe($cmd, $args))
    };

    // Inherit
    (inherit, $cmd:literal) => {
        exec_command!(inherit, $cmd, Vec::<String>::new())
    };
    (inherit, $cmd:expr, $args:expr) => {
        exec_command!(input, ExecCommandInput::inherit($cmd, $args))
    };

    // Legacy pipe
    ($cmd:literal) => {
        exec_command!(pipe, $cmd)
    };
    ($cmd:expr, [ $($arg:literal),* ]) => {
        exec_command!(pipe, $cmd, [ $($arg),* ])
    };
    ($cmd:expr, $args:expr) => {
        exec_command!(pipe, $cmd, $args)
    };
}

/// Calls the `send_request` host function to send an HTTP request
/// and return a response. Not OK responses must be handled by the guest.
#[macro_export]
macro_rules! send_request {
    (input, $input:expr) => {{
        #[allow(clippy::macro_metavars_in_unsafe)]
        let mut output = unsafe { send_request(Json($input))?.0 };
        populate_send_request_output(&mut output);
        output
    }};
    ($url:literal) => {
        send_request!(input, SendRequestInput::new($url))
    };
    ($url:expr) => {
        send_request!(input, SendRequestInput::new($url))
    };
}

/// Calls the `get_env_var` or `set_env_var` host function to manage
/// environment variables on the host.
///
/// When setting `PATH`, the provided value will append to `PATH`,
/// not overwrite it. Supports both `;` and `:` delimiters.
#[macro_export]
macro_rules! host_env {
    ($name:expr, $value:expr) => {
        unsafe { set_env_var($name.try_into()?, $value.try_into()?)? };
    };
    ($name:expr) => {
        unsafe {
            let inner = get_env_var($name.try_into()?)?;

            if inner.is_empty() { None } else { Some(inner) }
        }
    };
}

/// Calls the `host_log` host function to log a message to the host's terminal.
#[macro_export]
macro_rules! host_log {
    (input, $input:expr) => {
        unsafe {
            host_log!(Json($input))?;
        };
    };
    (stdout, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Stdout,
            ..HostLogInput::default()
        })
    };
    (stderr, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Stderr,
            ..HostLogInput::default()
        })
    };
    (error, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Error,
            ..HostLogInput::default()
        })
    };
    (warn, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Warn,
            ..HostLogInput::default()
        })
    };
    (debug, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Debug,
            ..HostLogInput::default()
        })
    };
    (trace, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Trace,
            ..HostLogInput::default()
        })
    };
    ($($arg:tt)+) => {
        host_log!(input, HostLogInput::new(format!($($arg)+)))
    };
}

/// Calls `from_virtual_path` on the host to convert the provided value to a real path
/// from a virtual path.
#[macro_export]
macro_rules! real_path {
    (buf, $path:expr) => {
        real_path!($path.to_string_lossy())
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
    ($path:expr) => {{
        let data = unsafe { to_virtual_path($path.try_into()?)? };
        let path: VirtualPath = json::from_str(&data)?;
        path
    }};
}
