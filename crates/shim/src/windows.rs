use command_group::CommandGroup;
use std::io;
use std::process::{exit, Command};

// On Windows, use job objects, as there's no way to replace the process.
pub fn exec_command_and_replace(mut command: Command) -> io::Result<()> {
    let mut group = command.group();
    group.kill_on_drop(true);

    let mut child = group.spawn()?;
    let status = child.wait()?;

    exit(status.code().unwrap_or(1))
}
