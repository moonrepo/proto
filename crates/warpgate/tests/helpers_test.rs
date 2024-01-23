use std::{collections::BTreeMap, path::PathBuf};
use warpgate::{from_virtual_path, to_virtual_path};

#[test]
fn converts_virtual_paths() {
    let paths_nix =
        BTreeMap::from_iter([(PathBuf::from("/Users/warp"), PathBuf::from("/userhome"))]);
    let paths_win =
        BTreeMap::from_iter([(PathBuf::from("C:\\Users\\warp"), PathBuf::from("/userhome"))]);

    // Match unix
    let a1 = to_virtual_path(&paths_nix, "/Users/warp/some/path");
    assert_eq!(a1.to_str().unwrap(), "/userhome/some/path");

    let a2 = from_virtual_path(&paths_nix, a1);
    assert_eq!(
        a2.to_str().unwrap(),
        if cfg!(windows) {
            "\\Users\\warp\\some\\path"
        } else {
            "/Users/warp/some/path"
        }
    );

    // Match windows
    let a1 = to_virtual_path(&paths_win, "C:\\Users\\warp\\some\\path");
    assert_eq!(a1.to_str().unwrap(), "/userhome/some/path");

    let a2 = from_virtual_path(&paths_win, a1);
    assert_eq!(
        a2.to_str().unwrap(),
        if cfg!(windows) {
            "C:\\Users\\warp\\some\\path"
        } else {
            "C:\\Users\\warp/some/path"
        }
    );

    // No match
    let b1 = to_virtual_path(&paths_nix, "/Unknown/prefix/some/path");
    assert_eq!(b1.to_str().unwrap(), "/Unknown/prefix/some/path");

    let b2 = from_virtual_path(&paths_nix, b1);
    assert_eq!(b2.to_str().unwrap(), "/Unknown/prefix/some/path");
}
