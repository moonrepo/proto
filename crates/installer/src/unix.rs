use starbase_utils::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn self_replace(
    current_exe: &Path,
    replacement_exe: &Path,
    relocate_to: &Path,
) -> miette::Result<()> {
    let mut exe = current_exe.to_path_buf();

    // If we're a symlink, we need to find the real location and operate on
    // that instead of the link.
    if let Ok(meta) = std::fs::symlink_metadata(&exe) {
        if meta.is_symlink() {
            exe = std::fs::read_link(exe).expect("TODO");
        }
    }

    let perms = fs::metadata(&exe)?.permissions();

    // Relocate the current executable. We do a rename/move as it keeps the
    // same inode's, just changes the literal path. This allows the binary
    // to keep executing without failure. A copy will *not* work!
    fs::rename(exe, relocate_to)?;

    // We then copy the replacement executable to the original location,
    // and attempt to persist the original permissions.
    fs::copy_file(replacement_exe, current_exe)?;
    fs::update_perms(current_exe, Some(perms.mode()))?;

    Ok(())
}
