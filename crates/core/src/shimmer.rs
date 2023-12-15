use starbase_utils::fs;
use std::path::Path;

#[cfg(debug_assertions)]
pub const SHIM_VERSION: u8 = 0;

#[cfg(not(debug_assertions))]
pub const SHIM_VERSION: u8 = 11;

#[cfg(not(windows))]
mod unix {
    use super::*;

    // On Unix, we can't copy or overwrite an executable that is currently running,
    // but we can remove the file (the i-node still exists) and create the new shim
    // alongside it.
    // @see https://groups.google.com/g/comp.unix.programmer/c/pUNlGCwJHK4?pli=1
    pub fn create_shim(
        source_code: &[u8],
        shim_path: &Path,
        find_only: bool,
    ) -> miette::Result<()> {
        if find_only && shim_path.exists() {
            return Ok(());
        }

        // Remove the current exe
        fs::remove_file(shim_path)?;

        // Create the new exe
        fs::write_file(shim_path, source_code)?;
        fs::update_perms(shim_path, None)?;

        Ok(())
    }

    pub fn get_shim_file_name(name: &str) -> String {
        name.to_owned()
    }
}

#[cfg(windows)]
mod windows {
    use super::*;

    // On Windows, we can't remove or overwrite an executable that is currently running,
    // but we can rename it and create the new shim alongside it.
    // @see https://stackoverflow.com/a/7198760
    pub fn create_shim(
        source_code: &[u8],
        shim_path: &Path,
        find_only: bool,
    ) -> miette::Result<()> {
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
        fs::write_file(shim_path, source_code)?;

        // Attempt to remove the old exe (but don't fail)
        if renamed_shim_path.exists() {
            let _ = fs::remove_file(renamed_shim_path);
        }

        Ok(())
    }

    pub fn get_shim_file_name(name: &str) -> String {
        format!("{name}.exe")
    }
}

#[cfg(not(windows))]
pub use unix::*;

#[cfg(windows)]
pub use windows::*;
