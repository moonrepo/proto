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
// but we can stage a new shim alongside it and atomically rename it into place.
// @see https://groups.google.com/g/comp.unix.programmer/c/pUNlGCwJHK4?pli=1
pub fn create_shim(source_code: &[u8], shim_path: &Path) -> io::Result<()> {
    let mut temp_shim_path = shim_path.to_path_buf();
    temp_shim_path.set_extension(format!("tmp-{}", std::process::id()));

    // Remove any stale temp file from a previous interrupted write
    if temp_shim_path.exists() {
        fs::remove_file(&temp_shim_path)?;
    }

    fs::write(&temp_shim_path, source_code)?;
    fs::set_permissions(&temp_shim_path, fs::Permissions::from_mode(0o755))?;

    if let Err(error) = fs::rename(&temp_shim_path, shim_path) {
        let _ = fs::remove_file(&temp_shim_path);

        return Err(error);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_dir(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);

        fs::create_dir_all(&path).unwrap();

        path
    }

    #[test]
    fn create_shim_replaces_existing_file() {
        let temp_dir = create_temp_dir("proto-shim-unix");
        let shim_path = temp_dir.join("node");

        fs::write(&shim_path, b"old").unwrap();

        create_shim(b"new", &shim_path).unwrap();

        assert_eq!(fs::read(&shim_path).unwrap(), b"new");
        assert_eq!(
            fs::metadata(&shim_path).unwrap().permissions().mode() & 0o777,
            0o755
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }
}
