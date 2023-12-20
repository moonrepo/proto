#[cfg(not(windows))]
mod unix;

#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
pub use unix::*;

#[cfg(windows)]
pub use windows::*;

use std::env;
use std::path::PathBuf;

pub fn locate_proto_bin(bin: &str) -> Option<PathBuf> {
    let bin = if cfg!(windows) {
        format!("{bin}.exe")
    } else {
        bin.to_owned()
    };

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

    if let Ok(dir) = env::var("PROTO_INSTALL_DIR") {
        lookup_dirs.push(PathBuf::from(dir));
    }

    if let Ok(dir) = env::var("PROTO_HOME") {
        lookup_dirs.push(PathBuf::from(dir).join("bin"));
    }

    // Special case for unit tests and other isolations where
    // PROTO_HOME is set to something random, but the proto
    // binaries still exist in their original location.
    if let Some(dir) = dirs::home_dir() {
        lookup_dirs.push(dir.join(".proto").join("bin"));
    }

    for lookup_dir in lookup_dirs {
        let file = lookup_dir.join(&bin);

        if file.is_absolute() && file.exists() {
            return Some(file);
        }
    }

    None
}
