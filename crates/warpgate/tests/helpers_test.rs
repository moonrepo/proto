use std::{collections::BTreeMap, path::PathBuf};
use warpgate::{from_virtual_path, to_virtual_path};

#[cfg(not(windows))]
#[test]
fn converts_virtual_paths() {
    let paths = BTreeMap::from_iter([(PathBuf::from("/Users/warp"), PathBuf::from("/userhome"))]);

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
    let paths =
        BTreeMap::from_iter([(PathBuf::from("C:\\Users\\warp"), PathBuf::from("/userhome"))]);

    // Match
    let a1 = to_virtual_path(&paths, "C:\\Users\\warp\\some\\path");
    assert_eq!(a1.to_string(), "/userhome/some/path");

    let a2 = from_virtual_path(&paths, a1);
    assert_eq!(a2.to_str().unwrap(), "C:\\Users\\warp\\some\\path");

    // No match
    let b1 = to_virtual_path(&paths, "C:\\Unknown\\prefix\\some\\path");
    assert_eq!(b1.to_string(), "C:\\Unknown\\prefix\\some\\path");

    let b2 = from_virtual_path(&paths, b1);
    assert_eq!(b2.to_str().unwrap(), "C:\\Unknown\\prefix\\some\\path");
}
