use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use warpgate_api::RealPath;

mod real_path {
    use super::*;

    mod construction {
        use super::*;

        #[test]
        fn creates_from_any_path_like_value() {
            assert_eq!(*RealPath::new("/foo/bar"), PathBuf::from("/foo/bar"));
            assert_eq!(
                *RealPath::new(Path::new("/foo/bar")),
                PathBuf::from("/foo/bar")
            );
            assert_eq!(
                *RealPath::new(PathBuf::from("/foo/bar")),
                PathBuf::from("/foo/bar")
            );
        }

        #[test]
        fn defaults_to_an_empty_path() {
            let path = RealPath::default();

            assert!(path.is_empty());
            assert_eq!(path, RealPath::new(""));
        }

        #[test]
        fn is_only_empty_for_empty_paths() {
            assert!(RealPath::new("").is_empty());
            assert!(!RealPath::new("/").is_empty());
            assert!(!RealPath::new("foo").is_empty());
        }

        #[test]
        fn into_inner_returns_the_inner_path() {
            assert_eq!(
                RealPath::new("/foo/bar").into_inner(),
                PathBuf::from("/foo/bar")
            );
        }
    }

    mod parent {
        use super::*;

        #[test]
        fn returns_the_parent_directory() {
            assert_eq!(
                RealPath::new("/foo/bar/baz").parent(),
                Some(RealPath::new("/foo/bar"))
            );
            assert_eq!(RealPath::new("/foo").parent(), Some(RealPath::new("/")));
        }

        #[test]
        fn returns_none_for_the_root_directory() {
            assert_eq!(RealPath::new("/").parent(), None);
        }

        #[test]
        fn returns_none_for_an_empty_path() {
            assert_eq!(RealPath::new("").parent(), None);
        }

        #[test]
        fn returns_an_empty_path_for_a_relative_file_name() {
            assert_eq!(RealPath::new("foo").parent(), Some(RealPath::default()));
        }
    }

    mod path_methods {
        use super::*;

        #[test]
        fn join_appends_relative_paths() {
            assert_eq!(
                RealPath::new("/foo").join("bar/baz"),
                RealPath::new("/foo/bar/baz")
            );
        }

        #[test]
        fn join_replaces_with_absolute_paths() {
            assert_eq!(
                RealPath::new("/foo").join("/other"),
                RealPath::new("/other")
            );
        }

        #[test]
        fn with_extension_replaces_the_extension() {
            assert_eq!(
                RealPath::new("/foo/file.txt").with_extension("rs"),
                RealPath::new("/foo/file.rs")
            );
            assert_eq!(
                RealPath::new("/foo/file").with_extension("rs"),
                RealPath::new("/foo/file.rs")
            );
            assert_eq!(
                RealPath::new("/foo/file.txt").with_extension(""),
                RealPath::new("/foo/file")
            );
        }

        #[test]
        fn with_added_extension_appends_an_extension() {
            assert_eq!(
                RealPath::new("/foo/file.tar").with_added_extension("gz"),
                RealPath::new("/foo/file.tar.gz")
            );
        }

        #[test]
        fn with_file_name_replaces_the_file_name() {
            assert_eq!(
                RealPath::new("/foo/file.txt").with_file_name("other.rs"),
                RealPath::new("/foo/other.rs")
            );
        }
    }

    mod traits {
        use super::*;

        #[test]
        fn displays_the_inner_path() {
            assert_eq!(RealPath::new("/foo/bar").to_string(), "/foo/bar");
        }

        #[test]
        fn derefs_to_path_methods() {
            let path = RealPath::new("/foo/bar.txt");

            assert_eq!(path.file_name(), Some(OsStr::new("bar.txt")));
            assert_eq!(path.extension(), Some(OsStr::new("txt")));
            assert!(path.starts_with("/foo"));
        }

        #[test]
        fn deref_mut_mutates_the_inner_path() {
            let mut path = RealPath::new("/foo");
            path.push("bar");

            assert_eq!(path, RealPath::new("/foo/bar"));

            path.set_extension("txt");

            assert_eq!(path, RealPath::new("/foo/bar.txt"));
        }

        #[test]
        fn converts_with_as_ref() {
            fn to_path(value: impl AsRef<Path>) -> PathBuf {
                value.as_ref().to_path_buf()
            }

            fn to_path_buf(value: impl AsRef<PathBuf>) -> PathBuf {
                value.as_ref().clone()
            }

            fn to_os_str(value: impl AsRef<OsStr>) -> OsString {
                value.as_ref().to_os_string()
            }

            fn to_self(value: impl AsRef<RealPath>) -> RealPath {
                value.as_ref().clone()
            }

            let path = RealPath::new("/foo/bar");

            assert_eq!(to_path(&path), PathBuf::from("/foo/bar"));
            assert_eq!(to_path_buf(&path), PathBuf::from("/foo/bar"));
            assert_eq!(to_os_str(&path), OsStr::new("/foo/bar"));
            assert_eq!(to_self(&path), path);
        }

        #[test]
        fn supports_eq_and_hash() {
            let mut set = HashSet::new();
            set.insert(RealPath::new("/foo"));
            set.insert(RealPath::new("/foo"));
            set.insert(RealPath::new("/bar"));

            assert_eq!(set.len(), 2);
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn serializes_to_a_string() {
            assert_eq!(
                serde_json::to_string(&RealPath::new("/foo/bar")).unwrap(),
                "\"/foo/bar\""
            );
        }

        #[test]
        fn deserializes_from_a_string() {
            let path: RealPath = serde_json::from_str(r#""/real/dir/file.txt""#).unwrap();

            assert_eq!(path, RealPath::new("/real/dir/file.txt"));
        }

        #[test]
        fn round_trips_through_json() {
            let path = RealPath::new("/real/dir/file.txt");
            let json = serde_json::to_string(&path).unwrap();

            assert_eq!(serde_json::from_str::<RealPath>(&json).unwrap(), path);
        }

        #[test]
        fn deserializes_from_a_virtual_shape_by_swapping_prefixes() {
            let path: RealPath = serde_json::from_str(
                r#"{"path":"/virtual/dir/file.txt","virtual_prefix":"/virtual","real_prefix":"/real"}"#,
            )
            .unwrap();

            assert_eq!(path, RealPath::new("/real/dir/file.txt"));
        }

        #[test]
        fn deserializes_the_virtual_prefix_itself() {
            let path: RealPath = serde_json::from_str(
                r#"{"path":"/virtual","virtual_prefix":"/virtual","real_prefix":"/real"}"#,
            )
            .unwrap();

            assert_eq!(path, RealPath::new("/real"));
        }

        #[test]
        fn deserializes_from_a_virtual_shape_with_aliases() {
            let path: RealPath =
                serde_json::from_str(r#"{"path":"/virtual/file.txt","v":"/virtual","r":"/real"}"#)
                    .unwrap();

            assert_eq!(path, RealPath::new("/real/file.txt"));
        }

        #[test]
        fn errors_when_the_virtual_prefix_does_not_match() {
            let error = serde_json::from_str::<RealPath>(
                r#"{"path":"/other/file.txt","virtual_prefix":"/virtual","real_prefix":"/real"}"#,
            )
            .unwrap_err();

            assert!(
                error
                    .to_string()
                    .contains("missing compatible virtual prefixes")
            );
        }
    }
}
