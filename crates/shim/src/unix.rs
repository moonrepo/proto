use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

// Use `execvp`, which replaces the current process. This helps
// thoroughly with signal handling, by passing them directly to the process.
// @see https://github.com/rust-lang/cargo/blob/master/crates/cargo-util/src/process_builder.rs#L572
pub fn exec_command_and_replace(mut command: Command) -> io::Result<()> {
    Err(command.exec())
}

// Return the file name as-is.
pub fn get_exe_file_name(name: &str) -> String {
    name.to_owned()
}

// Return the file name as-is.
pub fn get_shim_file_name(name: &str) -> String {
    name.to_owned()
}

// We can't copy or overwrite an executable that is currently running,
// but we can remove the file (the i-node still exists) and create the new shim
// alongside it.
// @see https://groups.google.com/g/comp.unix.programmer/c/pUNlGCwJHK4?pli=1
pub fn create_shim(source_code: &[u8], shim_path: &Path, find_only: bool) -> io::Result<()> {
    if find_only && shim_path.exists() {
        return Ok(());
    }

    // Remove the current exe
    if shim_path.exists() {
        fs::remove_file(shim_path)?;
    }

    // Create the new exe
    fs::write(shim_path, source_code)?;

    // And make it writable
    fs::set_permissions(shim_path, fs::Permissions::from_mode(0o755))?;

    Ok(())
}
