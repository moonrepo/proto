use std::ffi::OsStr;
use std::path::PathBuf;

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
