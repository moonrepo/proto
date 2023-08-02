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
    ($cmd:expr, [ $($arg:expr),* ]) => {
        unsafe {
          exec_command(Json(ExecCommandInput::new($cmd, [
              $($arg),*
          ])))?.0
        }
    };
    ($input:expr) => {
        unsafe { exec_command(Json($input))?.0 }
    };
}

#[macro_export]
macro_rules! host_log {
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
    ($($arg:tt)+) => {
        unsafe {
            host_log(Json(format!($($arg)+).into()))?;
        }
    };
}

#[macro_export]
macro_rules! return_err {
    ($msg:expr, $code:literal) => {
        Err(WithReturnCode::new(
            PluginError::Message($msg).into(),
            $code,
        ))
    };
    ($msg:expr) => {
        Err(WithReturnCode::new(PluginError::Message($msg).into(), 1))
    };
}
