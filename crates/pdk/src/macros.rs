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

#[macro_export]
macro_rules! exec_command {
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

#[macro_export]
macro_rules! host_log {
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
