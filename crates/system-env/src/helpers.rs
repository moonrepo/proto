use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

/// Return an absolute path to the provided program (without extension)
/// by checking `PATH` and cycling through `PATHEXT` extensions.
#[cfg(windows)]
pub fn find_command_on_path<T: AsRef<OsStr>>(name: T) -> Option<PathBuf> {
    use std::env;

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

            if path.exists() && path.is_file() {
                return Some(path);
            }
        } else {
            for ext in &exts {
                let mut file_name = name.to_os_string();
                file_name.push(ext);

                let path = path_dir.join(file_name);

                if path.exists() && path.is_file() {
                    return Some(path);
                }
            }
        }
    }

    None
}

/// Return an absolute path to the provided command by checking `PATH`.
#[cfg(unix)]
pub fn find_command_on_path<T: AsRef<OsStr>>(name: T) -> Option<PathBuf> {
    use std::env;

    let Ok(system_path) = env::var("PATH") else {
        return None;
    };

    let name = name.as_ref();

    for path_dir in env::split_paths(&system_path) {
        let path = path_dir.join(name);

        if path.exists() && path.is_file() {
            return Some(path);
        }
    }

    None
}

#[cfg(target_arch = "wasm32")]
pub fn find_command_on_path<T: AsRef<OsStr>>(_name: T) -> Option<PathBuf> {
    None
}

/// Return true if the provided command/program (without extension)
/// is available on `PATH`.
pub fn is_command_on_path<T: AsRef<OsStr>>(name: T) -> bool {
    find_command_on_path(name.as_ref()).is_some()
}

/// Create a new process [`Command`] and append the provided arguments. If the provided executable
/// name is not an absolute path, we'll attempt to find it on `PATH` using [`find_command_on_path`].
///
/// Furthermore, if the executable path is a Windows script (`.ps1`, `.cmd`, `.bat`), we'll wrap
/// the executable in a PowerShell command, and pass the original command via `-Command`.
pub fn create_process_command<T: AsRef<OsStr>, I: IntoIterator<Item = A>, A: AsRef<OsStr>>(
    exe: T,
    args: I,
) -> Command {
    let exe = exe.as_ref();

    // If an absolute path, use as-is, otherwise find the command
    let exe_path = if exe
        .as_encoded_bytes()
        .iter()
        .any(|b| b.eq_ignore_ascii_case(&b'/') || b.eq_ignore_ascii_case(&b'\\'))
    {
        PathBuf::from(exe)
    } else {
        find_command_on_path(exe).unwrap_or_else(|| exe.into())
    };

    create_process_command_from_path(exe_path, args)
}

fn create_process_command_from_path<I: IntoIterator<Item = A>, A: AsRef<OsStr>>(
    exe_path: PathBuf,
    args: I,
) -> Command {
    // If a Windows script, we must execute the command through powershell
    match exe_path.extension().and_then(|ext| ext.to_str()) {
        Some("ps1" | "cmd" | "bat") => {
            // This conversion is unfortunate...
            let args = args
                .into_iter()
                .map(|a| String::from_utf8_lossy(a.as_ref().as_encoded_bytes()).to_string())
                .collect::<Vec<_>>();

            let mut cmd =
                Command::new(find_command_on_path("pwsh").unwrap_or_else(|| "powershell".into()));
            cmd.arg("-Command");

            // Wrap the exe path in double quotes for PowerShell
            cmd.arg(
                format!(
                    "& \"{}\" {}",
                    exe_path.display().to_string().replace("\"", "`\""),
                    shell_words::join(args)
                )
                .trim(),
            );

            cmd
        }
        _ => {
            let mut cmd = Command::new(exe_path);
            cmd.args(args);
            cmd
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // https://github.com/moonrepo/proto/issues/960
    #[test]
    fn test_create_process_command_escapes_special_chars_in_path() {
        let exe_path =
            PathBuf::from(r"C:\Users\vbox)user\.proto\tools\pnpm\10.30.3\shims\pnpm.cmd");
        let args: Vec<&str> = vec!["--version"];

        let cmd = create_process_command_from_path(exe_path, args);

        let cmd_args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();

        // The -Command argument should have the exe path properly quoted
        // so that PowerShell doesn't choke on special characters like ")"
        assert_eq!(cmd_args.len(), 2);
        assert_eq!(cmd_args[0], "-Command");

        let command_str = &cmd_args[1];

        // The path must be wrapped with & '...' so PowerShell treats it
        // as a literal path even with special characters like ")"
        assert_eq!(
            command_str,
            r#"& "C:\Users\vbox)user\.proto\tools\pnpm\10.30.3\shims\pnpm.cmd" --version"#
        );
    }

    #[test]
    fn test_create_process_command_escapes_single_quotes_in_path() {
        let exe_path = PathBuf::from(r"C:\Users\O'Brien\.proto\tools\pnpm\10.30.3\shims\pnpm.cmd");
        let args: Vec<&str> = vec!["install"];

        let cmd = create_process_command_from_path(exe_path, args);

        let cmd_args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();

        assert_eq!(cmd_args[0], "-Command");

        assert_eq!(
            cmd_args[1],
            r#"& "C:\Users\O'Brien\.proto\tools\pnpm\10.30.3\shims\pnpm.cmd" install"#
        );
    }
}
