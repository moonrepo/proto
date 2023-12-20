use command_group::CommandGroup;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{exit, Command};

// Use job objects for process grouping, as there's no way to replace the process.
// @see https://github.com/rust-lang/cargo/blob/master/crates/cargo-util/src/process_builder.rs#L617
pub fn exec_command_and_replace(mut command: Command) -> io::Result<()> {
    let mut group = command.group();
    group.kill_on_drop(true);

    let mut child = group.spawn()?;
    let status = child.wait()?;

    exit(status.code().unwrap_or(1))
}

// Always use an `.exe` extension.
pub fn get_shim_file_name(name: &str) -> String {
    format!("{name}.exe")
}

// We can't remove or overwrite an executable that is currently running,
// but we can rename it and create the new shim alongside it.
// @see https://stackoverflow.com/a/7198760
pub fn create_shim(source_code: &[u8], shim_path: &Path, find_only: bool) -> io::Result<()> {
    if find_only && shim_path.exists() {
        return Ok(());
    }

    let mut renamed_shim_path = shim_path.to_path_buf();
    renamed_shim_path.set_extension("previous.exe");

    // Rename the current exe
    if shim_path.exists() {
        fs::rename(shim_path, &renamed_shim_path)?;
    }

    // Create the new exe
    fs::write(shim_path, source_code)?;

    // Attempt to remove the old exe (but don't fail)
    if renamed_shim_path.exists() {
        let _ = fs::remove_file(renamed_shim_path);
    }

    Ok(())
}
