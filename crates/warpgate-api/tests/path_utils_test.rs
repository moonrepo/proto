use std::path::PathBuf;
use warpgate_api::sort_paths_list;

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

#[cfg(not(windows))]
#[test]
fn converts_virtual_paths() {
    use warpgate_api::{convert_to_real_path, convert_to_virtual_path};

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
