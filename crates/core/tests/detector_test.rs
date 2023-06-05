use proto_core::*;
use rustc_hash::FxHashSet;
use starbase_sandbox::create_empty_sandbox;
use std::path::{Path, PathBuf};

pub fn create_manifest(dir: &Path, manifest: Manifest) -> PathBuf {
    let manifest_path = dir.join(MANIFEST_NAME);

    starbase_utils::json::write_file(&manifest_path, &manifest, true).unwrap();

    manifest_path
}

mod expanded_version {
    use super::*;

    #[test]
    fn returns_alias() {
        let temp = create_empty_sandbox();

        assert_eq!(
            expand_detected_version("unknown", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "unknown"
        );
    }

    #[test]
    fn handles_explicit() {
        let temp = create_empty_sandbox();

        assert_eq!(
            expand_detected_version("1.2.3-alpha", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        ); // ?
        assert_eq!(
            expand_detected_version("1.2.3", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        );
        assert_eq!(
            expand_detected_version("1.2.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.0"
        );
        assert_eq!(
            expand_detected_version("1.2", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2"
        );
        assert_eq!(
            expand_detected_version("1.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.0"
        );
        assert_eq!(
            expand_detected_version("1", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1"
        );

        assert_eq!(
            expand_detected_version("v1.2.3-alpha", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        ); // ?
        assert_eq!(
            expand_detected_version("V1.2.3", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        );
        assert_eq!(
            expand_detected_version("v1.2.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.0"
        );
        assert_eq!(
            expand_detected_version("V1.2", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2"
        );
        assert_eq!(
            expand_detected_version("v1.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.0"
        );
        assert_eq!(
            expand_detected_version("V1", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1"
        );
    }

    #[test]
    fn handles_equals() {
        let temp = create_empty_sandbox();

        assert_eq!(
            expand_detected_version("=1.2.3-alpha", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        ); // ?
        assert_eq!(
            expand_detected_version("=1.2.3", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        );
        assert_eq!(
            expand_detected_version("=1.2.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.0"
        );
        assert_eq!(
            expand_detected_version("=1.2", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2"
        );
        assert_eq!(
            expand_detected_version("=1.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.0"
        );
        assert_eq!(
            expand_detected_version("=1", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1"
        );
    }

    #[test]
    fn handles_star() {
        let temp = create_empty_sandbox();

        assert_eq!(
            expand_detected_version("=1.2.*", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2"
        );
        assert_eq!(
            expand_detected_version("=1.*", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1"
        );
        assert_eq!(
            expand_detected_version("1.2.*", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2"
        );
        assert_eq!(
            expand_detected_version("1.*", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1"
        );
    }

    #[test]
    fn handles_star_all() {
        let temp = create_empty_sandbox();

        let manifest_path = create_manifest(temp.path(), Manifest::default());

        assert_eq!(
            expand_detected_version("*", &Manifest::load(manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "latest"
        );

        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.2.3".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            expand_detected_version("*", &Manifest::load(manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        );
    }

    #[test]
    fn handles_caret() {
        let temp = create_empty_sandbox();
        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            expand_detected_version("^1.2.3-alpha", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version("^1.2.3", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version("^1.2.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version("^1.2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version("^1.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version("^1", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );

        // Failures
        assert_eq!(
            expand_detected_version("^1.6", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            expand_detected_version("^2", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            expand_detected_version("^0", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
    }

    #[test]
    fn handles_tilde() {
        let temp = create_empty_sandbox();
        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.2.9".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            expand_detected_version("~1.2.3-alpha", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.9"
        );
        assert_eq!(
            expand_detected_version("~1.2.3", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.9"
        );
        assert_eq!(
            expand_detected_version("~1.2.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.9"
        );
        assert_eq!(
            expand_detected_version("~1.2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.9"
        );
        assert_eq!(
            expand_detected_version("~1", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.9"
        );

        // Failures
        assert_eq!(
            expand_detected_version("~1.3", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            expand_detected_version("~1.1", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            expand_detected_version("~1.0", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            expand_detected_version("~2", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            expand_detected_version("~0", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
    }

    #[test]
    fn handles_gt() {
        let temp = create_empty_sandbox();
        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            expand_detected_version(">1.2.3-alpha", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">1.2.3", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">1.2.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">1.2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">1.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );

        // Failures
        assert_eq!(
            expand_detected_version(">1.6", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            expand_detected_version(">1.5.9", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            expand_detected_version(">2", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            expand_detected_version(">1", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
    }

    #[test]
    fn handles_gte() {
        let temp = create_empty_sandbox();
        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            expand_detected_version(">=1.2.3-alpha", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">=1.2.3", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">=1.2.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">=1.2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">=1.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">=1", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">=0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );

        // Failures
        assert_eq!(
            expand_detected_version(">1.6", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            expand_detected_version(">=2", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
    }

    #[test]
    fn handles_multi() {
        let temp = create_empty_sandbox();
        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            expand_detected_version("^1.2.3 || ^2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version("^1.6 || ^2", &Manifest::load(&manifest_path).unwrap())
                .unwrap(),
            None
        );
    }

    #[test]
    fn handles_range_with_space() {
        let temp = create_empty_sandbox();
        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            expand_detected_version(">=1.2.3 <2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            expand_detected_version(">=1.2.3, <2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
    }
}
