use process_wrap::std::*;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{exit, Command};

// Use job objects for process grouping, as there's no way to replace the process.
// @see https://github.com/rust-lang/cargo/blob/master/crates/cargo-util/src/process_builder.rs#L617
pub fn exec_command_and_replace(command: Command) -> io::Result<()> {
    let mut command = StdCommandWrap::from(command);
    command.wrap(JobObject);

    let mut child = command.spawn()?;
    let status = child.wait()?;

    exit(status.code().unwrap_or(1))
}

// Always use an `.exe` extension.
pub fn get_exe_file_name(name: &str) -> String {
    if name.ends_with(".exe") {
        name.to_owned()
    } else {
        format!("{name}.exe")
    }
}

// Always use an `.exe` extension.
pub fn get_shim_file_name(name: &str) -> String {
    get_exe_file_name(name)
}

macro_rules! handle_io_error {
    ($expr:expr) => {
        if let Err(error) = $expr {
            // If we receive an "Access is denied" error, we should
            // exit early as there's no way around this, as this exe
            // may be currently in use by another process. This happens
            // consistently when ran through task runners (like moon).
            if error.raw_os_error().is_some_and(|code| code == 5) {
                return Ok(());
            } else {
                return Err(error);
            }
        }
    };
}

// We can't remove or overwrite an executable that is currently running,
// but we can rename it and create the new shim alongside it.
// @see https://stackoverflow.com/a/7198760
pub fn create_shim(source_code: &[u8], shim_path: &Path) -> io::Result<()> {
    let mut renamed_shim_path = shim_path.to_path_buf();
    renamed_shim_path.set_extension("previous.exe");

    // Attempt to remove the old exe (but don't fail)
    if renamed_shim_path.exists() {
        let _ = fs::remove_file(&renamed_shim_path);
    }

    // Rename the current exe
    if shim_path.exists() {
        handle_io_error!(fs::rename(shim_path, &renamed_shim_path));
    }

    // Create the new exe
    handle_io_error!(fs::write(shim_path, source_code));

    // Attempt to remove the old exe (but don't fail)
    if renamed_shim_path.exists() {
        let _ = fs::remove_file(renamed_shim_path);
    }

    Ok(())
}
