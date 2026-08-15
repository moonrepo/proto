/// Return an error message wrapped in [`WithReturnCode`](extism_pdk::WithReturnCode),
/// for use within [`#[plugin_fn]`](macro@extism_pdk::plugin_fn).
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

/// Calls the `download_file` host function to download a file from a URL
/// and save it to a destination file on the host machine.
#[macro_export]
macro_rules! download_file {
    (input, $input:expr) => {{
        #[allow(clippy::macro_metavars_in_unsafe)]
        unsafe {
            download_file(Json($input))?.0
        }
    }};
    ($url:expr, $file:expr) => {
        download_file!(input, DownloadFileInput::new($url, $file))
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
            host_log(Json($input))?;
        };
    };
    (stdout, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Stdout,
            ..Default::default()
        })
    };
    (stderr, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Stderr,
            ..Default::default()
        })
    };
    (error, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Error,
            ..Default::default()
        })
    };
    (warn, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Warn,
            ..Default::default()
        })
    };
    (debug, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Debug,
            ..Default::default()
        })
    };
    (trace, $($arg:tt)+) => {
        host_log!(input, HostLogInput {
            message: format!($($arg)+),
            target: HostLogTarget::Trace,
            ..Default::default()
        })
    };
    ($($arg:tt)+) => {
        host_log!(input, HostLogInput::new(format!($($arg)+)))
    };
}
