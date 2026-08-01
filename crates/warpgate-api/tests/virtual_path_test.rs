use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use warpgate_api::VirtualPath;

mod virtual_path {
    use super::*;

    mod construction {
        use super::*;

        #[test]
        fn creates_from_any_path_like_value() {
            assert_eq!(*VirtualPath::new("/foo/bar"), PathBuf::from("/foo/bar"));
            assert_eq!(
                *VirtualPath::new(Path::new("/foo/bar")),
                PathBuf::from("/foo/bar")
            );
            assert_eq!(
                *VirtualPath::new(PathBuf::from("/foo/bar")),
                PathBuf::from("/foo/bar")
            );
        }

        #[test]
        fn defaults_to_an_empty_path() {
            let path = VirtualPath::default();

            assert!(path.is_empty());
            assert_eq!(path, VirtualPath::new(""));
        }

        #[test]
        fn is_only_empty_for_empty_paths() {
            assert!(VirtualPath::new("").is_empty());
            assert!(!VirtualPath::new("/").is_empty());
            assert!(!VirtualPath::new("foo").is_empty());
        }

        #[test]
        fn into_inner_returns_the_inner_path() {
            assert_eq!(
                VirtualPath::new("/foo/bar").into_inner(),
                PathBuf::from("/foo/bar")
            );
        }
    }

    mod parent {
        use super::*;

        #[test]
        fn returns_the_parent_directory() {
            assert_eq!(
                VirtualPath::new("/foo/bar/baz").parent(),
                Some(VirtualPath::new("/foo/bar"))
            );
            assert_eq!(
                VirtualPath::new("/foo").parent(),
                Some(VirtualPath::new("/"))
            );
        }

        #[test]
        fn returns_none_for_the_root_directory() {
            assert_eq!(VirtualPath::new("/").parent(), None);
        }

        #[test]
        fn returns_none_for_an_empty_path() {
            assert_eq!(VirtualPath::new("").parent(), None);
        }

        #[test]
        fn returns_an_empty_path_for_a_relative_file_name() {
            assert_eq!(
                VirtualPath::new("foo").parent(),
                Some(VirtualPath::default())
            );
        }
    }

    mod path_methods {
        use super::*;

        #[test]
        fn join_appends_relative_paths() {
            assert_eq!(
                VirtualPath::new("/foo").join("bar/baz"),
                VirtualPath::new("/foo/bar/baz")
            );
        }

        #[test]
        fn join_replaces_with_absolute_paths() {
            assert_eq!(
                VirtualPath::new("/foo").join("/other"),
                VirtualPath::new("/other")
            );
        }

        #[test]
        fn with_extension_replaces_the_extension() {
            assert_eq!(
                VirtualPath::new("/foo/file.txt").with_extension("rs"),
                VirtualPath::new("/foo/file.rs")
            );
            assert_eq!(
                VirtualPath::new("/foo/file").with_extension("rs"),
                VirtualPath::new("/foo/file.rs")
            );
            assert_eq!(
                VirtualPath::new("/foo/file.txt").with_extension(""),
                VirtualPath::new("/foo/file")
            );
        }

        #[test]
        fn with_added_extension_appends_an_extension() {
            assert_eq!(
                VirtualPath::new("/foo/file.tar").with_added_extension("gz"),
                VirtualPath::new("/foo/file.tar.gz")
            );
        }

        #[test]
        fn with_file_name_replaces_the_file_name() {
            assert_eq!(
                VirtualPath::new("/foo/file.txt").with_file_name("other.rs"),
                VirtualPath::new("/foo/other.rs")
            );
        }
    }

    mod traits {
        use super::*;

        #[test]
        fn displays_the_inner_path() {
            assert_eq!(VirtualPath::new("/foo/bar").to_string(), "/foo/bar");
        }

        #[test]
        fn derefs_to_path_methods() {
            let path = VirtualPath::new("/foo/bar.txt");

            assert_eq!(path.file_name(), Some(OsStr::new("bar.txt")));
            assert_eq!(path.extension(), Some(OsStr::new("txt")));
            assert!(path.starts_with("/foo"));
        }

        #[test]
        fn deref_mut_mutates_the_inner_path() {
            let mut path = VirtualPath::new("/foo");
            path.push("bar");

            assert_eq!(path, VirtualPath::new("/foo/bar"));

            path.set_extension("txt");

            assert_eq!(path, VirtualPath::new("/foo/bar.txt"));
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

            fn to_self(value: impl AsRef<VirtualPath>) -> VirtualPath {
                value.as_ref().clone()
            }

            let path = VirtualPath::new("/foo/bar");

            assert_eq!(to_path(&path), PathBuf::from("/foo/bar"));
            assert_eq!(to_path_buf(&path), PathBuf::from("/foo/bar"));
            assert_eq!(to_os_str(&path), OsStr::new("/foo/bar"));
            assert_eq!(to_self(&path), path);
        }

        #[test]
        fn supports_eq_and_hash() {
            let mut set = HashSet::new();
            set.insert(VirtualPath::new("/foo"));
            set.insert(VirtualPath::new("/foo"));
            set.insert(VirtualPath::new("/bar"));

            assert_eq!(set.len(), 2);
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn serializes_to_a_string() {
            assert_eq!(
                serde_json::to_string(&VirtualPath::new("/foo/bar")).unwrap(),
                "\"/foo/bar\""
            );
        }

        #[test]
        fn deserializes_from_a_virtual_shape() {
            let path: VirtualPath = serde_json::from_str(
                r#"{"path":"/virtual/dir/file.txt","virtual_prefix":"/virtual","real_prefix":"/real"}"#,
            )
            .unwrap();

            assert_eq!(path, VirtualPath::new("/virtual/dir/file.txt"));
        }

        #[test]
        fn deserializes_from_a_virtual_shape_with_aliases() {
            let path: VirtualPath =
                serde_json::from_str(r#"{"path":"/virtual/file.txt","v":"/virtual","r":"/real"}"#)
                    .unwrap();

            assert_eq!(path, VirtualPath::new("/virtual/file.txt"));
        }

        // A real path cannot be converted to a virtual path, as the
        // shape provides no prefixes to convert with.
        #[test]
        fn errors_when_deserializing_from_a_string() {
            let error = serde_json::from_str::<VirtualPath>(r#""/real/dir/file.txt""#).unwrap_err();

            assert!(
                error
                    .to_string()
                    .contains("do not have access to path prefixes")
            );
        }
    }
}
