// This code is shared between the shim and main binaries!

use std::io;
use std::process::{Command, ExitStatus};

// On Unix, use `execvp`, which replaces the current process.
#[cfg(not(windows))]
pub fn spawn_command_and_replace(mut command: Command) -> io::Result<ExitStatus> {
    use std::os::unix::process::CommandExt;

    Err(command.exec())
}

// On Windows, use job objects.
#[cfg(windows)]
pub fn spawn_command_and_replace(mut command: Command) -> io::Result<ExitStatus> {
    use command_group::CommandGroup;

    let mut group = command.group();
    group.kill_on_drop(true);

    let mut child = group.spawn()?;
    child.wait()
}
