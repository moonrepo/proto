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
    (pipe, $cmd:expr, $args:expr) => {
        unsafe {
          exec_command(Json(ExecCommandInput::pipe($cmd, $args)))?.0
        }
    };
    (inherit, $cmd:expr, $args:expr) => {
        unsafe {
          exec_command(Json(ExecCommandInput::inherit($cmd, $args)))?.0
        }
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
