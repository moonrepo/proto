use starbase_archive::Archiver;
use starbase_archive::codecs::Gz;
use starbase_archive::tar::TarPacker;
use starbase_sandbox::create_empty_sandbox;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use warpgate::{
    WarpgateLoaderError, extract_file_name_from_url, from_virtual_path, move_or_unpack_file,
    sort_virtual_paths, to_virtual_path,
};

// A WASM binary's first 4 bytes are the magic header `\0asm`. Used so the
// file is treated as a real WASM (not an empty file) by anything that sniffs.
const WASM_MAGIC: &[u8] = b"\0asm\x01\x00\x00\x00";

// Pack the provided files into a `.tar.gz` archive at `archive_path`.
// Each `(rel_path, contents)` entry is added relative to `source_root`.
fn pack_tar_gz(source_root: &Path, archive_path: &Path, files: &[(&str, &[u8])]) {
    for (rel, contents) in files {
        let target = source_root.join(rel);
        fs::write_file(&target, contents).unwrap();
    }

    let mut archiver = Archiver::new(source_root, archive_path);

    for (rel, _) in files {
        archiver.add_source_file(rel, None);
    }

    archiver
        .pack(|path| Ok(TarPacker::new(Gz::new(fs::create_file(path).unwrap()))))
        .unwrap();
}

#[test]
fn sorts_virtual_paths() {
    let mut paths = vec![
        (PathBuf::from("/Users/warp"), PathBuf::from("/userhome")),
        (PathBuf::from("/Users/warp/.proto"), PathBuf::from("/proto")),
        (
            PathBuf::from("/Users/warp/.proto/temp"),
            PathBuf::from("/temp"),
        ),
        (
            PathBuf::from("/Users/warp/Projects/moon/example"),
            PathBuf::from("/workspace"),
        ),
        (
            PathBuf::from("/Users/warp/Projects/other/length"),
            PathBuf::from("/workdir"),
        ),
        (PathBuf::from("/Other/path"), PathBuf::from("/cwd")),
    ];

    sort_virtual_paths(&mut paths);

    assert_eq!(
        paths
            .iter()
            .map(|(h, g)| (h.to_str().unwrap(), g.to_str().unwrap()))
            .collect::<Vec<_>>(),
        [
            ("/Users/warp/Projects/other/length", "/workdir"),
            ("/Users/warp/Projects/moon/example", "/workspace"),
            ("/Users/warp/.proto/temp", "/temp"),
            ("/Users/warp/.proto", "/proto"),
            ("/Users/warp", "/userhome"),
            ("/Other/path", "/cwd")
        ]
    );
}

#[cfg(not(windows))]
#[test]
fn converts_virtual_paths() {
    let paths = vec![(PathBuf::from("/Users/warp"), PathBuf::from("/userhome"))];

    // Match
    let a1 = to_virtual_path(&paths, "/Users/warp/some/path");
    assert_eq!(a1.to_string(), "/userhome/some/path");

    let a2 = from_virtual_path(&paths, a1);
    assert_eq!(a2.to_str().unwrap(), "/Users/warp/some/path");

    // No match
    let b1 = to_virtual_path(&paths, "/Unknown/prefix/some/path");
    assert_eq!(b1.to_string(), "/Unknown/prefix/some/path");

    let b2 = from_virtual_path(&paths, b1);
    assert_eq!(b2.to_str().unwrap(), "/Unknown/prefix/some/path");
}

#[cfg(windows)]
#[test]
fn converts_virtual_paths() {
    let paths = vec![(PathBuf::from("C:\\Users\\warp"), PathBuf::from("/userhome"))];

    // Match
    let a1 = to_virtual_path(&paths, "C:\\Users\\warp\\some\\path");
    assert_eq!(a1.to_string(), "/userhome/some/path");

    let a2 = from_virtual_path(&paths, a1);
    assert_eq!(a2.to_str().unwrap(), "C:\\Users\\warp\\some\\path");

    // No match
    let b1 = to_virtual_path(&paths, "C:\\Unknown\\prefix\\some\\path");
    assert_eq!(b1.to_string(), "C:/Unknown/prefix/some/path");

    let b2 = from_virtual_path(&paths, b1);
    assert_eq!(b2.to_str().unwrap(), "C:\\Unknown\\prefix\\some\\path");
}

mod extract_file_name_from_url {
    use super::*;

    #[test]
    fn returns_last_path_segment_for_valid_url() {
        assert_eq!(
            extract_file_name_from_url("https://example.com/path/file.tar.gz"),
            "file.tar.gz"
        );
        assert_eq!(
            extract_file_name_from_url("https://example.com/plugin.wasm"),
            "plugin.wasm"
        );
    }

    // url::Url::parse fails for inputs without a scheme. The fallback then
    // splits on the rightmost slash.
    #[test]
    fn falls_back_to_rightmost_slash_for_non_url() {
        assert_eq!(
            extract_file_name_from_url("not a url/file.wasm"),
            "file.wasm"
        );
    }

    #[test]
    fn returns_unknown_when_no_segments_or_slashes() {
        assert_eq!(extract_file_name_from_url(""), "unknown");
        assert_eq!(extract_file_name_from_url("just-a-string"), "unknown");
    }
}

mod move_or_unpack_file {
    use super::*;

    #[test]
    fn unpacks_archive_and_prefers_release_dir() {
        let sandbox = create_empty_sandbox();
        let source_root = sandbox.path().join("src");
        let archive_path = sandbox.path().join("plugin.tar.gz");
        let mut dest_path = sandbox.path().join("plugin.wasm");

        // Two `.wasm` files: one in `release/` and one outside. The function
        // should prefer the release one so that archives that include build
        // artefacts under `target/release/` resolve correctly.
        pack_tar_gz(
            &source_root,
            &archive_path,
            &[
                ("debug/plugin.wasm", b"not this one"),
                ("release/plugin.wasm", WASM_MAGIC),
            ],
        );

        move_or_unpack_file(&archive_path, &mut dest_path, &["wasm".into()]).unwrap();

        assert!(dest_path.exists());

        let bytes = fs::read_file_bytes(&dest_path).unwrap();
        assert_eq!(bytes, WASM_MAGIC);
    }

    #[test]
    fn unpacks_archive_with_no_release_dir() {
        let sandbox = create_empty_sandbox();
        let source_root = sandbox.path().join("src");
        let archive_path = sandbox.path().join("plugin.tar.gz");
        let mut dest_path = sandbox.path().join("plugin.wasm");

        pack_tar_gz(&source_root, &archive_path, &[("plugin.wasm", WASM_MAGIC)]);

        move_or_unpack_file(&archive_path, &mut dest_path, &["wasm".into()]).unwrap();

        let bytes = fs::read_file_bytes(&dest_path).unwrap();
        assert_eq!(bytes, WASM_MAGIC);
    }

    #[test]
    fn errors_when_archive_contains_no_wasm() {
        let sandbox = create_empty_sandbox();
        let source_root = sandbox.path().join("src");
        let archive_path = sandbox.path().join("plugin.tar.gz");
        let mut dest_path = sandbox.path().join("plugin.wasm");

        pack_tar_gz(
            &source_root,
            &archive_path,
            &[("README.md", b"no wasm here")],
        );

        let err = move_or_unpack_file(&archive_path, &mut dest_path, &["wasm".into()]).unwrap_err();

        assert!(
            matches!(err, WarpgateLoaderError::NoWasmFound { .. }),
            "expected NoWasmFound, got: {err:?}"
        );
        assert!(!dest_path.exists());
    }

    #[test]
    fn renames_plain_wasm_file() {
        let sandbox = create_empty_sandbox();
        let temp_path = sandbox.path().join("temp.wasm");
        let mut dest_path = sandbox.path().join("plugin.wasm");

        fs::write_file(&temp_path, WASM_MAGIC).unwrap();

        move_or_unpack_file(&temp_path, &mut dest_path, &["wasm".into()]).unwrap();

        assert!(dest_path.exists());
        // `fs::rename` moves the file — temp should no longer exist.
        assert!(!temp_path.exists());

        let bytes = fs::read_file_bytes(&dest_path).unwrap();
        assert_eq!(bytes, WASM_MAGIC);
    }

    #[test]
    fn errors_on_unsupported_extension() {
        let sandbox = create_empty_sandbox();
        let temp_path = sandbox.path().join("temp.wasm");
        let mut dest_path = sandbox.path().join("plugin.exe");

        fs::write_file(&temp_path, b"not a plugin").unwrap();

        let err = move_or_unpack_file(&temp_path, &mut dest_path, &["wasm".into()]).unwrap_err();

        assert!(
            matches!(
                err,
                WarpgateLoaderError::UnsupportedDownloadExtension { ref ext, .. } if ext == "exe"
            ),
            "expected UnsupportedDownloadExtension(exe), got: {err:?}"
        );
        assert!(!dest_path.exists());
    }

    #[test]
    fn errors_on_missing_extension() {
        let sandbox = create_empty_sandbox();
        let temp_path = sandbox.path().join("temp.wasm");
        let mut dest_path = sandbox.path().join("plugin");

        fs::write_file(&temp_path, b"unknown").unwrap();

        let err = move_or_unpack_file(&temp_path, &mut dest_path, &["wasm".into()]).unwrap_err();

        assert!(
            matches!(err, WarpgateLoaderError::UnknownDownloadType { .. }),
            "expected UnknownDownloadType, got: {err:?}"
        );
        assert!(!dest_path.exists());
    }

    // When the caller permits multiple extensions and the archive holds a
    // non-WASM plugin (e.g. a TOML config layer shipped as `.tar.gz`), the
    // function must:
    //   1. find the .toml file inside the archive
    //   2. mutate `dest_path` from the `.wasm` placeholder to `.toml` so the
    //      cache lookup on the next load finds the right file
    //   3. write the file's actual contents
    // This exercises the mutable-dest contract added when extensions became
    // caller-configurable.
    #[test]
    fn unpacks_non_wasm_archive_and_updates_dest_extension() {
        let sandbox = create_empty_sandbox();
        let source_root = sandbox.path().join("src");
        let archive_path = sandbox.path().join("plugin.tar.gz");
        let mut dest_path = sandbox.path().join("plugin.wasm");

        pack_tar_gz(
            &source_root,
            &archive_path,
            &[("plugin.toml", b"[plugin]\nname = \"test\"\n")],
        );

        move_or_unpack_file(
            &archive_path,
            &mut dest_path,
            &["wasm".into(), "toml".into()],
        )
        .unwrap();

        assert_eq!(dest_path.extension().and_then(|e| e.to_str()), Some("toml"));
        assert!(dest_path.exists());

        let bytes = fs::read_file_bytes(&dest_path).unwrap();
        assert!(bytes.starts_with(b"[plugin]"));
    }

    // With both `.wasm` and `.toml` in an archive, the function must honor
    // the caller's extension priority (first wins) — i.e. `.wasm` is picked
    // and `dest_path` is left at the `.wasm` placeholder.
    #[test]
    fn prefers_first_extension_when_archive_contains_multiple() {
        let sandbox = create_empty_sandbox();
        let source_root = sandbox.path().join("src");
        let archive_path = sandbox.path().join("plugin.tar.gz");
        let mut dest_path = sandbox.path().join("plugin.wasm");

        pack_tar_gz(
            &source_root,
            &archive_path,
            &[
                ("plugin.toml", b"[plugin]\nname = \"test\"\n"),
                ("plugin.wasm", WASM_MAGIC),
            ],
        );

        move_or_unpack_file(
            &archive_path,
            &mut dest_path,
            &["wasm".into(), "toml".into()],
        )
        .unwrap();

        assert_eq!(dest_path.extension().and_then(|e| e.to_str()), Some("wasm"));
        let bytes = fs::read_file_bytes(&dest_path).unwrap();
        assert_eq!(bytes, WASM_MAGIC);
    }
}
