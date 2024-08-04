use starbase_utils::fs::{self, FsError};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn self_replace(
    current_exe: &Path,
    replace_with: &Path,
    relocate_to: &Path,
) -> miette::Result<()> {
    // If we're a symlink, we need to find the real location and operate on
    // that instead of the link.
    let exe = current_exe.canonicalize().map_err(|error| FsError::Read {
        path: current_exe.to_path_buf(),
        error: Box::new(error),
    })?;
    let perms = fs::metadata(&exe)?.permissions();

    // Relocate the current executable. We do a rename/move as it keeps the
    // same inode's, just changes the literal path. This allows the binary
    // to keep executing without failure. A copy will *not* work!
    fs::rename(exe, relocate_to)?;

    // We then copy the replacement executable to the original location,
    // and attempt to persist the original permissions.
    fs::copy_file(replace_with, current_exe)?;
    fs::update_perms(current_exe, Some(perms.mode()))?;

    Ok(())
}
