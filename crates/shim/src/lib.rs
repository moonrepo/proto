#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::*;
#[cfg(windows)]
pub use windows::*;

use std::env;
use std::path::PathBuf;

pub const SHIM_VERSION: u8 = 19;

pub fn locate_proto_exe(exe_name: &str) -> Option<PathBuf> {
    let exe_name = get_exe_file_name(exe_name);
    let mut lookup_dirs = vec![];

    // When in development, ensure we're using the target built proto,
    // and not the proto available globally on `PATH`.
    #[cfg(any(debug_assertions, test))]
    {
        if let Ok(dir) = env::var("CARGO_TARGET_DIR") {
            lookup_dirs.push(PathBuf::from(dir).join("debug"));
        }

        if let Ok(dir) = env::var("CARGO_MANIFEST_DIR") {
            lookup_dirs.push(
                PathBuf::from(if let Some(index) = dir.find("crates") {
                    &dir[0..index]
                } else {
                    &dir
                })
                .join("target")
                .join("debug"),
            );
        }

        if let Ok(dir) = env::var("GITHUB_WORKSPACE") {
            lookup_dirs.push(PathBuf::from(dir).join("target").join("debug"));
        }

        if let Ok(dir) = env::current_dir() {
            lookup_dirs.push(dir.join("target").join("debug"));
        }
    }

    if let Ok(dir) = env::var("PROTO_HOME") {
        let dir = PathBuf::from(dir);

        if let Ok(version) = env::var("PROTO_VERSION") {
            lookup_dirs.push(dir.join("tools").join("proto").join(version));
        }

        lookup_dirs.push(dir.join("bin"));
    }

    if let Ok(dir) = env::var("PROTO_LOOKUP_DIR") {
        lookup_dirs.push(dir.into());
    }

    // Detect the currently running executable (proto), and then find
    // a proto-shim sibling in the same directory. This assumes both
    // binaries are the same version.
    if let Ok(current) = env::current_exe() {
        if let Some(dir) = current.parent() {
            lookup_dirs.push(dir.to_path_buf());
        }
    }

    // Special case for unit tests and other isolations where
    // PROTO_HOME is set to something random, but the proto
    // binaries still exist in their original location.
    if let Some(dir) = dirs::home_dir() {
        if let Ok(version) = env::var("PROTO_VERSION") {
            lookup_dirs.push(dir.join(".proto").join("tools").join("proto").join(version));
        }

        lookup_dirs.push(dir.join(".proto").join("bin"));
    }

    for lookup_dir in lookup_dirs {
        let file = lookup_dir.join(&exe_name);

        if file.is_absolute() && file.is_file() {
            return Some(file);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvGuard {
        key: &'static str,
        value: Option<OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let previous = env::var_os(key);

            unsafe {
                env::set_var(key, value);
            }

            Self {
                key,
                value: previous,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.value {
                unsafe {
                    env::set_var(self.key, value);
                }
            } else {
                unsafe {
                    env::remove_var(self.key);
                }
            }
        }
    }

    fn create_temp_dir(name: &str) -> PathBuf {
        let unique = format!(
            "{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let path = env::temp_dir().join(unique);

        fs::create_dir_all(&path).unwrap();

        path
    }

    #[test]
    fn ignores_directory_candidates_when_locating_proto_exe() {
        let _lock = env_lock().lock().unwrap();
        let lookup_dir = create_temp_dir("proto-shim-locate-dir");
        let exe_name = format!("proto-locate-dir-{}", std::process::id());
        let candidate_dir = lookup_dir.join(get_exe_file_name(&exe_name));
        let _guard = EnvGuard::set("PROTO_LOOKUP_DIR", &lookup_dir);

        fs::create_dir_all(&candidate_dir).unwrap();

        assert_eq!(locate_proto_exe(&exe_name), None);

        fs::remove_dir_all(lookup_dir).unwrap();
    }

    #[test]
    fn locates_files_from_lookup_dir() {
        let _lock = env_lock().lock().unwrap();
        let lookup_dir = create_temp_dir("proto-shim-locate-file");
        let exe_name = format!("proto-locate-file-{}", std::process::id());
        let candidate_file = lookup_dir.join(get_exe_file_name(&exe_name));
        let _guard = EnvGuard::set("PROTO_LOOKUP_DIR", &lookup_dir);

        fs::write(&candidate_file, b"proto").unwrap();

        assert_eq!(locate_proto_exe(&exe_name), Some(candidate_file));

        fs::remove_dir_all(lookup_dir).unwrap();
    }
}
