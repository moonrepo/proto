use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

// On Unix, use `execvp`, which replaces the current process. This helps
// thoroughly with signal handling, by passing them directly to the process.
pub fn exec_command_and_replace(mut command: Command) -> io::Result<()> {
    Err(command.exec())
}
