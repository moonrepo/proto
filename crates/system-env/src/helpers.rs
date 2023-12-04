use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

/// Return an absolute path to the provided program (without extension)
/// by checking `PATH` and cycling through `PATHEXT` extensions.
#[cfg(windows)]
pub fn find_command_on_path<T: AsRef<OsStr>>(name: T) -> Option<PathBuf> {
    let Ok(system_path) = env::var("PATH") else {
        return None;
    };

    // Only extensions we care about
    let exts = vec![".exe", ".ps1", ".cmd", ".bat"];
    let name = name.as_ref();
    let has_ext = name
        .as_encoded_bytes()
        .iter()
        .any(|b| b.eq_ignore_ascii_case(&b'.'));

    for path_dir in env::split_paths(&system_path) {
        if has_ext {
            let path = path_dir.join(name);

            if path.exists() {
                return Some(path);
            }
        } else {
            for ext in &exts {
                let mut file_name = name.to_os_string();
                file_name.push(ext);

                let path = path_dir.join(file_name);

                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    None
}

/// Return an absolute path to the provided command by checking `PATH`.
#[cfg(not(windows))]
pub fn find_command_on_path<T: AsRef<OsStr>>(name: T) -> Option<PathBuf> {
    let Ok(system_path) = env::var("PATH") else {
        return None;
    };

    let name = name.as_ref();

    for path_dir in env::split_paths(&system_path) {
        let path = path_dir.join(name);

        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Return true if the provided command/program (without extension)
/// is available on `PATH`.
pub fn is_command_on_path<T: AsRef<OsStr>>(name: T) -> bool {
    find_command_on_path(name.as_ref()).is_some()
}

/// Create a new process [`Command`] and append the provided arguments. If the provided binary
/// name is not an absolute path, we'll attempt to find it on `PATH` using [`find_command_on_path`].
///
/// Furthermore, if the binary path is a Windows script (`.ps1`, `.cmd`, `.bat`), we'll wrap
/// the binary in a PowerShell command, and pass the original command via `-Command`.
pub fn create_process_command<T: AsRef<OsStr>, I: IntoIterator<Item = A>, A: AsRef<OsStr>>(
    bin: T,
    args: I,
) -> Command {
    let bin = bin.as_ref();

    // If an absolute path, use as-is, otherwise find the command
    let bin_path = if bin
        .as_encoded_bytes()
        .iter()
        .any(|b| b.eq_ignore_ascii_case(&b'/') || b.eq_ignore_ascii_case(&b'\\'))
    {
        PathBuf::from(bin)
    } else {
        find_command_on_path(bin).unwrap_or_else(|| bin.into())
    };

    let bin_ext = bin_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase());

    // If a Windows script, we must execute the command through powershell
    match bin_ext.as_deref() {
        Some("ps1" | "cmd" | "bat") => {
            // This conversion is unfortunate...
            let args = args
                .into_iter()
                .map(|a| String::from_utf8_lossy(a.as_ref().as_encoded_bytes()).to_string())
                .collect::<Vec<_>>();

            let mut cmd =
                Command::new(find_command_on_path("pwsh").unwrap_or_else(|| "powershell".into()));
            cmd.arg("-Command");
            cmd.arg(format!("{} {}", bin_path.display(), shell_words::join(args)).trim());
            cmd
        }
        _ => {
            let mut cmd = Command::new(bin_path);
            cmd.args(args);
            cmd
        }
    }
}
