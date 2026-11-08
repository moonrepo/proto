use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

// Use `execvp`, which replaces the current process. This helps
// thoroughly with signal handling, by passing them directly to the process.
// @see https://github.com/rust-lang/cargo/blob/master/crates/cargo-util/src/process_builder.rs#L572
pub fn exec_command_and_replace(mut command: Command) -> io::Result<()> {
    Err(command.exec())
}

// Return the file name as-is.
pub fn get_exe_file_name(name: &str) -> String {
    name.to_owned()
}

// Return the file name as-is.
pub fn get_shim_file_name(name: &str) -> String {
    name.to_owned()
}

// Shims only differ by file name, so link them to the source executable to
// avoid storing a copy for every shim.
fn link_or_copy_shim(source_path: &Path, shim_path: &Path) -> io::Result<()> {
    // Hard linking a symlink creates another symlink, so resolve it first.
    // Links also share permissions with the source, which we shouldn't change,
    // so only link when the source is already executable.
    if let Ok(real_path) = source_path.canonicalize()
        && fs::metadata(&real_path).is_ok_and(|meta| meta.permissions().mode() & 0o111 == 0o111)
        && fs::hard_link(&real_path, shim_path).is_ok()
    {
        return Ok(());
    }

    // Linking isn't always possible, for example, when the source
    // is on another device, or is owned by another user.
    fs::copy(source_path, shim_path)?;
    fs::set_permissions(shim_path, fs::Permissions::from_mode(0o755))
}

// We can't copy or overwrite an executable that is currently running,
// but we can stage a new shim alongside it and atomically rename it into place.
// @see https://groups.google.com/g/comp.unix.programmer/c/pUNlGCwJHK4?pli=1
pub fn create_shim(source_path: &Path, shim_path: &Path) -> io::Result<()> {
    let mut temp_shim_path = shim_path.to_path_buf();
    temp_shim_path.set_extension(format!("tmp-{}", std::process::id()));

    // Remove any stale temp file from a previous interrupted write
    if temp_shim_path.exists() {
        fs::remove_file(&temp_shim_path)?;
    }

    let result = link_or_copy_shim(source_path, &temp_shim_path)
        .and_then(|_| fs::rename(&temp_shim_path, shim_path));

    // Always clean up, as a rename does nothing and leaves
    // the temp file behind if both are links to the same file
    let _ = fs::remove_file(&temp_shim_path);

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{MetadataExt, symlink};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_dir(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);

        fs::create_dir_all(&path).unwrap();

        path
    }

    fn write_executable(path: &Path, contents: &[u8]) {
        fs::write(path, contents).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();

        names.sort();
        names
    }

    #[test]
    fn create_shim_replaces_existing_file() {
        let temp_dir = create_temp_dir("proto-shim-unix");
        let source_path = temp_dir.join("proto-shim");
        let shim_path = temp_dir.join("node");

        write_executable(&source_path, b"new");
        fs::write(&shim_path, b"old").unwrap();

        create_shim(&source_path, &shim_path).unwrap();

        assert_eq!(fs::read(&shim_path).unwrap(), b"new");
        assert_eq!(
            fs::metadata(&shim_path).unwrap().permissions().mode() & 0o777,
            0o755
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn create_shim_hard_links_source() {
        let temp_dir = create_temp_dir("proto-shim-unix-link");
        let source_path = temp_dir.join("proto-shim");
        let shim_path = temp_dir.join("node");

        write_executable(&source_path, b"shim");

        create_shim(&source_path, &shim_path).unwrap();

        let source = fs::metadata(&source_path).unwrap();
        let shim = fs::metadata(&shim_path).unwrap();

        assert_eq!(shim.ino(), source.ino());
        assert_eq!(shim.nlink(), 2);

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn create_shim_recreates_existing_link_without_leftovers() {
        let temp_dir = create_temp_dir("proto-shim-unix-relink");
        let source_path = temp_dir.join("proto-shim");
        let shim_path = temp_dir.join("node");

        write_executable(&source_path, b"shim");

        create_shim(&source_path, &shim_path).unwrap();
        create_shim(&source_path, &shim_path).unwrap();

        assert_eq!(file_names(&temp_dir), ["node", "proto-shim"]);
        assert_eq!(fs::metadata(&shim_path).unwrap().nlink(), 2);

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn create_shim_resolves_symlinked_source() {
        let temp_dir = create_temp_dir("proto-shim-unix-symlink");
        let real_path = temp_dir.join("proto-shim-real");
        let source_path = temp_dir.join("proto-shim");
        let shim_path = temp_dir.join("node");

        write_executable(&real_path, b"shim");
        symlink(&real_path, &source_path).unwrap();

        create_shim(&source_path, &shim_path).unwrap();

        // Must be a real file, otherwise the shim's name resolves to the source
        assert!(!shim_path.is_symlink());
        assert_eq!(
            fs::metadata(&shim_path).unwrap().ino(),
            fs::metadata(&real_path).unwrap().ino()
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn create_shim_copies_non_executable_source() {
        let temp_dir = create_temp_dir("proto-shim-unix-copy");
        let source_path = temp_dir.join("proto-shim");
        let shim_path = temp_dir.join("node");

        fs::write(&source_path, b"shim").unwrap();
        fs::set_permissions(&source_path, fs::Permissions::from_mode(0o644)).unwrap();

        create_shim(&source_path, &shim_path).unwrap();

        let source = fs::metadata(&source_path).unwrap();
        let shim = fs::metadata(&shim_path).unwrap();

        assert_ne!(shim.ino(), source.ino());
        assert_eq!(fs::read(&shim_path).unwrap(), b"shim");
        assert_eq!(shim.permissions().mode() & 0o777, 0o755);
        assert_eq!(source.permissions().mode() & 0o777, 0o644);

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn create_shim_errors_for_missing_source() {
        let temp_dir = create_temp_dir("proto-shim-unix-missing");
        let shim_path = temp_dir.join("node");

        assert!(create_shim(&temp_dir.join("proto-shim"), &shim_path).is_err());
        assert_eq!(file_names(&temp_dir), Vec::<String>::new());

        fs::remove_dir_all(temp_dir).unwrap();
    }
}
