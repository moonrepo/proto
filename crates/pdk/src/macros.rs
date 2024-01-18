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
