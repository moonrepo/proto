use starbase_utils::fs::{self, FsError};
use std::path::Path;

pub fn self_replace(
    current_exe: &Path,
    replace_with: &Path,
    relocate_to: &Path,
) -> miette::Result<()> {
    // If we're a symlink, we need to find the real location and operate on
    // that instead of the link.
    let mut exe = current_exe.canonicalize().map_err(|error| FsError::Read {
        path: current_exe.to_path_buf(),
        error: Box::new(error),
    })?;

    // Relocate the current executable. We do a rename/move as it keeps the
    // same ID/handle, just changes the literal path. This allows the binary
    // to keep executing without failure. A copy will *not* work!
    fs::rename(exe, relocate_to)?;

    // We then copy the replacement executable to a temporary location.
    let mut temp_exe = current_exe.to_path_buf();
    temp_exe.set_extension("temp.exe");

    fs::copy_file(replace_with, temp_exe)?;

    // And lastly, we move the temporary to the original location. This avoids
    // writing/copying data to the original, and instead does a rename/move.
    fs::rename(temp_exe, current_exe)?;

    Ok(())
}
