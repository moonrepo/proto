use starbase_utils::fs;
use std::path::Path;

#[cfg(debug_assertions)]
pub const SHIM_VERSION: u8 = 0;

#[cfg(not(debug_assertions))]
pub const SHIM_VERSION: u8 = 11;

pub fn create_shim(source_code: &[u8], shim_path: &Path, find_only: bool) -> miette::Result<()> {
    if find_only && shim_path.exists() {
        return Ok(());
    }

    fs::write_file(shim_path, source_code)?;
    fs::update_perms(shim_path, None)?;

    Ok(())
}

#[cfg(windows)]
pub fn get_shim_file_name(name: &str) -> String {
    format!("{name}.exe")
}

#[cfg(not(windows))]
pub fn get_shim_file_name(name: &str) -> String {
    name.to_owned()
}
