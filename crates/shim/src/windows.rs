use std::fs;
use std::io::{self, Error};
use std::path::Path;
use std::process::{Command, exit};
use windows_sys::Win32::Foundation::{FALSE, TRUE};
use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
use windows_sys::core::BOOL;

// @see https://github.com/rust-lang/cargo/blob/master/crates/cargo-util/src/process_builder.rs#L605
unsafe extern "system" fn ctrlc_handler(_: u32) -> BOOL {
    // Do nothing; let the child process handle it.
    TRUE
}

// Do "nothing", since Windows sends CTRL-C/BREAK to all processes connected
// to the current console. However, we don't want the shim process to capture it,
// but the underlying process should, so try and pass it through.
pub fn exec_command_and_replace(mut command: Command) -> io::Result<()> {
    unsafe {
        if SetConsoleCtrlHandler(Some(ctrlc_handler), TRUE) == FALSE {
            return Err(Error::other("Could not set Ctrl-C handler."));
        }
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            unsafe {
                let _ = SetConsoleCtrlHandler(Some(ctrlc_handler), FALSE);
            }

            return Err(error);
        }
    };

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

// Shims only differ by file name, so link them to the source executable to
// avoid storing a copy for every shim.
fn link_or_copy_shim(source_path: &Path, shim_path: &Path) -> io::Result<()> {
    // Hard linking a symlink creates another symlink, so resolve it first
    let linked = source_path
        .canonicalize()
        .and_then(|real_path| fs::hard_link(real_path, shim_path));

    // Linking isn't always possible, for example, when the source is on
    // another volume, or the file system or our access doesn't allow it.
    if linked.is_err() {
        fs::copy(source_path, shim_path)?;
    }

    Ok(())
}

// We can't remove or overwrite an executable that is currently running,
// but we can rename it and create the new shim alongside it.
// @see https://stackoverflow.com/a/7198760
pub fn create_shim(source_path: &Path, shim_path: &Path) -> io::Result<()> {
    let mut renamed_shim_path = shim_path.to_path_buf();
    renamed_shim_path.set_extension("previous.exe");

    // Attempt to remove the old exe (but don't fail)
    if renamed_shim_path.exists() {
        let _ = fs::remove_file(&renamed_shim_path);
    }

    // Rename the current exe
    let mut renamed_existing = false;

    if shim_path.exists() {
        fs::rename(shim_path, &renamed_shim_path)?;
        renamed_existing = true;
    }

    // Create the new exe
    if let Err(error) = link_or_copy_shim(source_path, shim_path) {
        if renamed_existing {
            let _ = fs::rename(&renamed_shim_path, shim_path);
        }

        return Err(error);
    }

    // Attempt to remove the old exe (but don't fail)
    if renamed_shim_path.exists() {
        let _ = fs::remove_file(renamed_shim_path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_dir(name: &str) -> PathBuf {
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
        let temp_dir = create_temp_dir("proto-shim-windows");
        let source_path = temp_dir.join("proto-shim.exe");
        let shim_path = temp_dir.join("node.exe");

        fs::write(&source_path, b"new").unwrap();
        fs::write(&shim_path, b"old").unwrap();

        create_shim(&source_path, &shim_path).unwrap();

        assert_eq!(fs::read(&shim_path).unwrap(), b"new");
        assert!(!temp_dir.join("node.previous.exe").exists());

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn create_shim_hard_links_source() {
        let temp_dir = create_temp_dir("proto-shim-windows-link");
        let source_path = temp_dir.join("proto-shim.exe");
        let shim_path = temp_dir.join("node.exe");

        fs::write(&source_path, b"before").unwrap();

        create_shim(&source_path, &shim_path).unwrap();

        // Writing in place is only visible through a link to the same file
        fs::write(&source_path, b"after").unwrap();

        assert_eq!(fs::read(&shim_path).unwrap(), b"after");

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn create_shim_recreates_existing_link() {
        let temp_dir = create_temp_dir("proto-shim-windows-relink");
        let source_path = temp_dir.join("proto-shim.exe");
        let shim_path = temp_dir.join("node.exe");

        fs::write(&source_path, b"shim").unwrap();

        create_shim(&source_path, &shim_path).unwrap();
        create_shim(&source_path, &shim_path).unwrap();

        assert_eq!(fs::read(&shim_path).unwrap(), b"shim");
        assert!(!temp_dir.join("node.previous.exe").exists());

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn create_shim_restores_existing_file_on_error() {
        let temp_dir = create_temp_dir("proto-shim-windows-missing");
        let shim_path = temp_dir.join("node.exe");

        fs::write(&shim_path, b"old").unwrap();

        assert!(create_shim(&temp_dir.join("proto-shim.exe"), &shim_path).is_err());
        assert_eq!(fs::read(&shim_path).unwrap(), b"old");

        fs::remove_dir_all(temp_dir).unwrap();
    }
}
