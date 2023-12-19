// This code is shared between the shim and main binaries!

use std::io;
use std::process::Command;

// On Unix, use `execvp`, which replaces the current process.
#[cfg(not(windows))]
pub fn spawn_command_and_replace(mut command: Command) -> io::Result<()> {
    use std::os::unix::process::CommandExt;

    Err(command.exec())
}

// On Windows, use job objects.
#[cfg(windows)]
pub fn spawn_command_and_replace(mut command: Command) -> io::Result<()> {
    use command_group::CommandGroup;

    let mut group = command.group();
    group.kill_on_drop(true);

    let mut child = group.spawn()?;
    let status = child.wait()?;

    std::process::exit(status.code().unwrap_or(1))
}
