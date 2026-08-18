use std::path::PathBuf;
use warpgate_api::{
    PathParseError, RealPath, VirtualPath, convert_to_real_native_path, convert_to_real_path,
    convert_to_virtual_path, sort_paths_list,
};

#[test]
fn sorts_paths() {
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

    sort_paths_list(&mut paths);

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

#[test]
fn sorts_equal_host_paths_by_guest_path() {
    let mut paths = vec![
        (PathBuf::from("/Users/warp"), PathBuf::from("/a")),
        (PathBuf::from("/Users/warp"), PathBuf::from("/b")),
    ];

    sort_paths_list(&mut paths);

    assert_eq!(
        paths,
        vec![
            (PathBuf::from("/Users/warp"), PathBuf::from("/b")),
            (PathBuf::from("/Users/warp"), PathBuf::from("/a")),
        ]
    );
}

#[cfg(not(windows))]
#[test]
fn converts_virtual_paths() {
    let paths = vec![(PathBuf::from("/Users/warp"), PathBuf::from("/userhome"))];

    // Match
    let a1 = convert_to_virtual_path("/Users/warp/some/path", &paths).unwrap();
    assert_eq!(a1.to_string_lossy(), "/userhome/some/path");

    let a2 = convert_to_real_path(a1, &paths).unwrap();
    assert_eq!(a2.to_str().unwrap(), "/Users/warp/some/path");

    // No match
    assert!(convert_to_virtual_path("/Unknown/prefix/some/path", &paths).is_none());
    assert!(convert_to_real_path("/unknown", &paths).is_none());
}

#[cfg(windows)]
#[test]
fn converts_virtual_paths() {
    let paths = vec![(PathBuf::from("C:\\Users\\warp"), PathBuf::from("/userhome"))];

    // Match
    let a1 = convert_to_virtual_path("C:\\Users\\warp\\some\\path", &paths).unwrap();
    assert_eq!(a1.to_string_lossy(), "/userhome/some/path");

    let a2 = convert_to_real_path(a1, &paths).unwrap();
    assert_eq!(a2.to_str().unwrap(), "C:\\Users\\warp\\some\\path");

    // No match
    assert!(convert_to_virtual_path("C:\\Unknown\\prefix\\some\\path", &paths).is_none());
    assert!(convert_to_real_path("/unknown", &paths).is_none());
}

#[test]
fn converts_paths_already_on_the_target_side() {
    let paths = vec![(PathBuf::from("/Users/warp"), PathBuf::from("/userhome"))];

    // Guest path stays a guest path
    assert_eq!(
        convert_to_virtual_path("/userhome/some/path", &paths).unwrap(),
        VirtualPath::new("/userhome/some/path")
    );

    // Host path stays a host path
    assert_eq!(
        convert_to_real_path("/Users/warp/some/path", &paths).unwrap(),
        RealPath::new("/Users/warp/some/path")
    );
}

#[test]
fn converts_the_prefixes_themselves() {
    let paths = vec![(PathBuf::from("/Users/warp"), PathBuf::from("/userhome"))];

    assert_eq!(
        convert_to_virtual_path("/Users/warp", &paths).unwrap(),
        VirtualPath::new("/userhome")
    );

    assert_eq!(
        convert_to_real_path("/userhome", &paths).unwrap(),
        RealPath::new("/Users/warp")
    );
}

// `PathBuf` equality ignores trailing separators, so these must compare
// the raw string form to catch a `join("")` on the stripped prefix.
// https://github.com/moonrepo/moon/issues/2676
#[cfg(not(windows))]
#[test]
fn converts_the_prefixes_themselves_without_trailing_separators() {
    let paths = vec![(PathBuf::from("/Users/warp"), PathBuf::from("/userhome"))];

    assert_eq!(
        convert_to_virtual_path("/Users/warp", &paths)
            .unwrap()
            .to_string_lossy(),
        "/userhome"
    );

    assert_eq!(
        convert_to_real_path("/userhome", &paths)
            .unwrap()
            .to_string_lossy(),
        "/Users/warp"
    );

    assert_eq!(
        convert_to_real_native_path("/userhome", &paths).to_string_lossy(),
        "/Users/warp"
    );

    // Also when the input itself has a trailing separator
    assert_eq!(
        convert_to_real_path("/userhome/", &paths)
            .unwrap()
            .to_string_lossy(),
        "/Users/warp"
    );
}

#[cfg(windows)]
#[test]
fn converts_the_prefixes_themselves_without_trailing_separators() {
    let paths = vec![(PathBuf::from("C:\\Users\\warp"), PathBuf::from("/userhome"))];

    assert_eq!(
        convert_to_virtual_path("C:\\Users\\warp", &paths)
            .unwrap()
            .to_string_lossy(),
        "/userhome"
    );

    assert_eq!(
        convert_to_real_path("/userhome", &paths)
            .unwrap()
            .to_string_lossy(),
        "C:\\Users\\warp"
    );

    assert_eq!(
        convert_to_real_native_path("/userhome", &paths).to_string_lossy(),
        "C:\\Users\\warp"
    );

    // Also when the input itself has a trailing separator
    assert_eq!(
        convert_to_real_path("/userhome/", &paths)
            .unwrap()
            .to_string_lossy(),
        "C:\\Users\\warp"
    );
}

// Entries are matched in order, which is why lists should be pre-sorted
// with `sort_paths_list` so that the longest prefix wins.
#[test]
fn converts_using_the_first_matching_entry() {
    let mut paths = vec![
        (PathBuf::from("/Users/warp"), PathBuf::from("/userhome")),
        (PathBuf::from("/Users/warp/.proto"), PathBuf::from("/proto")),
    ];

    assert_eq!(
        convert_to_virtual_path("/Users/warp/.proto/some/path", &paths).unwrap(),
        VirtualPath::new("/userhome/.proto/some/path")
    );

    sort_paths_list(&mut paths);

    assert_eq!(
        convert_to_virtual_path("/Users/warp/.proto/some/path", &paths).unwrap(),
        VirtualPath::new("/proto/some/path")
    );
}

#[test]
fn converts_native_paths() {
    let paths = vec![(PathBuf::from("/Users/warp"), PathBuf::from("/userhome"))];

    // Guest path becomes a host path
    assert_eq!(
        convert_to_real_native_path("/userhome/some/path", &paths),
        PathBuf::from("/Users/warp/some/path")
    );

    // Host path stays a host path
    assert_eq!(
        convert_to_real_native_path("/Users/warp/some/path", &paths),
        PathBuf::from("/Users/warp/some/path")
    );
}

// Unlike `convert_to_real_path`, the original path is
// returned instead of `None`.
#[test]
fn returns_original_native_path_when_nothing_matches() {
    let paths = vec![(PathBuf::from("/Users/warp"), PathBuf::from("/userhome"))];

    assert_eq!(
        convert_to_real_native_path("/Unknown/some/path", &paths),
        PathBuf::from("/Unknown/some/path")
    );
    assert_eq!(
        convert_to_real_native_path("/Unknown/some/path", &[]),
        PathBuf::from("/Unknown/some/path")
    );
}

#[test]
fn displays_path_parse_errors() {
    assert_eq!(
        PathParseError("something failed".into()).to_string(),
        "something failed"
    );
}
