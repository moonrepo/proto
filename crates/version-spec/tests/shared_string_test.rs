use compact_str::CompactString;
use std::collections::{BTreeSet, HashSet};
use version_spec::*;

mod shared_string {
    use super::*;

    const LONG: &str = "build.0123456789abcdef0123456789abcdef";

    #[test]
    fn converts_from_strings() {
        assert_eq!(SharedString::new("alpha").as_str(), "alpha");
        assert_eq!(SharedString::from("alpha").as_str(), "alpha");
        assert_eq!(SharedString::from(&String::from("alpha")).as_str(), "alpha");
        assert_eq!(SharedString::from(String::from("alpha")).as_str(), "alpha");
        assert_eq!(
            SharedString::from(CompactString::new("alpha")).as_str(),
            "alpha"
        );
        assert_eq!(SharedString::from(LONG).as_str(), LONG);
        assert_eq!(SharedString::from(String::from(LONG)).as_str(), LONG);
    }

    #[test]
    fn converts_into_strings() {
        let value = SharedString::from("alpha");

        assert_eq!(String::from(value.clone()), "alpha");
        assert_eq!(CompactString::from(value.clone()), "alpha");

        // Converts while another clone shares the string
        assert_eq!(CompactString::from(value), "alpha");
    }

    #[test]
    fn derefs_to_str() {
        let value = SharedString::from("alpha.1");

        assert_eq!(value.len(), 7);
        assert!(value.starts_with("alpha"));
        assert_eq!(AsRef::<str>::as_ref(&value), "alpha.1");
        assert_eq!(Some(value).as_deref(), Some("alpha.1"));
    }

    #[test]
    fn formats_like_str() {
        let value = SharedString::from("alpha");

        assert_eq!(value.to_string(), "alpha");
        assert_eq!(format!("{value:>7}"), "  alpha");
        assert_eq!(format!("{value:?}"), "\"alpha\"");
    }

    #[test]
    fn compares_with_strings() {
        let value = SharedString::from("alpha");

        assert_eq!(value, "alpha");
        assert_eq!(value, *"alpha");
        assert_eq!(value, String::from("alpha"));
        assert_eq!(value, CompactString::new("alpha"));
        assert_eq!("alpha", value);
        assert_eq!(*"alpha", value);
        assert_eq!(String::from("alpha"), value);
        assert_eq!(CompactString::new("alpha"), value);

        assert_ne!(value, "beta");
        assert_ne!(value, SharedString::from("beta"));
        assert_eq!(value, value.clone());
        assert_eq!(value, SharedString::from("alpha"));
    }

    #[test]
    fn orders_like_str() {
        let mut values = vec![
            SharedString::from("beta"),
            SharedString::from(LONG),
            SharedString::from("alpha.10"),
            SharedString::from("alpha.2"),
        ];

        values.sort();

        assert_eq!(values, ["alpha.10", "alpha.2", "beta", LONG]);
    }

    #[test]
    fn looks_up_by_str() {
        let hashed = HashSet::from([SharedString::from("alpha"), SharedString::from(LONG)]);
        let ordered = BTreeSet::from([SharedString::from("alpha"), SharedString::from(LONG)]);

        assert!(hashed.contains("alpha"));
        assert!(hashed.contains(LONG));
        assert!(!hashed.contains("beta"));
        assert!(ordered.contains("alpha"));
        assert!(ordered.contains(LONG));
        assert!(!ordered.contains("beta"));
    }

    #[test]
    fn serializes_as_string() {
        let value = SharedString::from("alpha");

        assert_eq!(serde_json::to_string(&value).unwrap(), "\"alpha\"");
        assert_eq!(
            serde_json::from_str::<SharedString>("\"alpha\"").unwrap(),
            value
        );
        assert!(serde_json::from_str::<SharedString>("1").is_err());
    }

    #[test]
    fn shares_with_clones_of_versions() {
        let version = Version::parse("vendor-1.2.3-alpha.1+build.5").unwrap();
        let req = version.to_requirement(Op::Exact);

        assert_eq!(version.scope.as_deref(), Some("vendor"));
        assert_eq!(version.prerelease.as_deref(), Some("alpha.1"));
        assert_eq!(version.build.as_deref(), Some("build.5"));
        assert_eq!(req.scope, version.scope);
        assert_eq!(req.prerelease, version.prerelease);
        assert!(req.matches(&version));

        let mut other = version.clone();
        other.prerelease = Some("alpha.2".into());

        assert!(!req.matches(&other));
        assert!(version < other);
    }
}
