use proto_core::*;
use std::path::{Path, PathBuf};

pub fn create_temp_dir() -> assert_fs::TempDir {
    assert_fs::TempDir::new().unwrap()
}

pub fn create_manifest(dir: &Path, manifest: Manifest) -> PathBuf {
    let manifest_path = dir.join(MANIFEST_NAME);

    starbase_utils::json::write_file(&manifest_path, &manifest, true).unwrap();

    manifest_path
}

mod fixed_version {
    use super::*;
    use rustc_hash::FxHashSet;

    #[test]
    fn ignores_invalid() {
        let temp = create_temp_dir();

        assert_eq!(
            detect_fixed_version("unknown", &Manifest::load_from(temp.path()).unwrap()).unwrap(),
            None
        );
    }

    #[test]
    fn handles_explicit() {
        let temp = create_temp_dir();

        assert_eq!(
            detect_fixed_version("1.2.3-alpha", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        ); // ?
        assert_eq!(
            detect_fixed_version("1.2.3", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        );
        assert_eq!(
            detect_fixed_version("1.2.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.0"
        );
        assert_eq!(
            detect_fixed_version("1.2", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2"
        );
        assert_eq!(
            detect_fixed_version("1.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.0"
        );
        assert_eq!(
            detect_fixed_version("1", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1"
        );

        assert_eq!(
            detect_fixed_version("v1.2.3-alpha", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        ); // ?
        assert_eq!(
            detect_fixed_version("V1.2.3", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        );
        assert_eq!(
            detect_fixed_version("v1.2.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.0"
        );
        assert_eq!(
            detect_fixed_version("V1.2", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2"
        );
        assert_eq!(
            detect_fixed_version("v1.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.0"
        );
        assert_eq!(
            detect_fixed_version("V1", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1"
        );
    }

    #[test]
    fn handles_equals() {
        let temp = create_temp_dir();

        assert_eq!(
            detect_fixed_version("=1.2.3-alpha", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        ); // ?
        assert_eq!(
            detect_fixed_version("=1.2.3", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        );
        assert_eq!(
            detect_fixed_version("=1.2.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.0"
        );
        assert_eq!(
            detect_fixed_version("=1.2", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2"
        );
        assert_eq!(
            detect_fixed_version("=1.0", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.0"
        );
        assert_eq!(
            detect_fixed_version("=1", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1"
        );
    }

    #[test]
    fn handles_star() {
        let temp = create_temp_dir();

        assert_eq!(
            detect_fixed_version("=1.2.*", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2"
        );
        assert_eq!(
            detect_fixed_version("=1.*", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1"
        );
        assert_eq!(
            detect_fixed_version("1.2.*", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1.2"
        );
        assert_eq!(
            detect_fixed_version("1.*", &Manifest::load_from(temp.path()).unwrap())
                .unwrap()
                .unwrap(),
            "1"
        );
    }

    #[test]
    fn handles_star_all() {
        let temp = create_temp_dir();

        let manifest_path = create_manifest(temp.path(), Manifest::default());

        assert_eq!(
            detect_fixed_version("*", &Manifest::load(manifest_path).unwrap()).unwrap(),
            None
        );

        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.2.3".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            detect_fixed_version("*", &Manifest::load(manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.3"
        );
    }

    #[test]
    fn handles_caret() {
        let temp = create_temp_dir();
        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            detect_fixed_version("^1.2.3-alpha", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version("^1.2.3", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version("^1.2.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version("^1.2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version("^1.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version("^1", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );

        // Failures
        assert_eq!(
            detect_fixed_version("^1.6", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            detect_fixed_version("^2", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            detect_fixed_version("^0", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
    }

    #[test]
    fn handles_tilde() {
        let temp = create_temp_dir();
        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.2.9".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            detect_fixed_version("~1.2.3-alpha", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.9"
        );
        assert_eq!(
            detect_fixed_version("~1.2.3", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.9"
        );
        assert_eq!(
            detect_fixed_version("~1.2.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.9"
        );
        assert_eq!(
            detect_fixed_version("~1.2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.9"
        );
        assert_eq!(
            detect_fixed_version("~1", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.2.9"
        );

        // Failures
        assert_eq!(
            detect_fixed_version("~1.3", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            detect_fixed_version("~1.1", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            detect_fixed_version("~1.0", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            detect_fixed_version("~2", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            detect_fixed_version("~0", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
    }

    #[test]
    fn handles_gt() {
        let temp = create_temp_dir();
        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            detect_fixed_version(">1.2.3-alpha", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version(">1.2.3", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version(">1.2.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version(">1.2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version(">1.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version(">0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );

        // Failures
        assert_eq!(
            detect_fixed_version(">1.6", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            detect_fixed_version(">1.5.9", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            detect_fixed_version(">2", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            detect_fixed_version(">1", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
    }

    #[test]
    fn handles_gte() {
        let temp = create_temp_dir();
        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            detect_fixed_version(">=1.2.3-alpha", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version(">=1.2.3", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version(">=1.2.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version(">=1.2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version(">=1.0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version(">=1", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version(">=0", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );

        // Failures
        assert_eq!(
            detect_fixed_version(">1.6", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
        assert_eq!(
            detect_fixed_version(">=2", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
    }

    #[test]
    fn handles_multi() {
        let temp = create_temp_dir();
        let manifest_path = create_manifest(
            temp.path(),
            Manifest {
                installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                ..Manifest::default()
            },
        );

        assert_eq!(
            detect_fixed_version("^1.2.3 || ^2", &Manifest::load(&manifest_path).unwrap())
                .unwrap()
                .unwrap(),
            "1.5.9"
        );
        assert_eq!(
            detect_fixed_version("^1.6 || ^2", &Manifest::load(&manifest_path).unwrap()).unwrap(),
            None
        );
    }
}
