use std::env;
use std::path::PathBuf;

/// Return an absolute path to the provided program (without extension)
/// by checking `PATH` and cycling through `PATHEXT` extensions.
#[cfg(windows)]
pub fn find_command_on_path(name: &str) -> Option<PathBuf> {
    let Ok(system_path) = env::var("PATH") else {
        return None;
    };
    let Ok(path_ext) = env::var("PATHEXT") else {
        return None;
    };
    let exts = path_ext.split(';').collect::<Vec<_>>();

    for path_dir in env::split_paths(&system_path) {
        for ext in &exts {
            let path = path_dir.join(format!("{name}{ext}"));

            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

/// Return an absolute path to the provided command by checking `PATH`.
#[cfg(not(windows))]
pub fn find_command_on_path(name: &str) -> Option<PathBuf> {
    let Ok(system_path) = env::var("PATH") else {
        return None;
    };

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
pub fn is_command_on_path(name: &str) -> bool {
    find_command_on_path(name).is_some()
}
