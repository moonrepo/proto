use proto_core::layout::Inventory;
use proto_pdk_api::LoadVersionsOutput;
use starbase_sandbox::create_empty_sandbox;
use std::path::Path;
use version_spec::VersionSpec;

fn create_inventory(dir: &Path) -> Inventory {
    Inventory {
        dir: dir.to_path_buf(),
        ..Default::default()
    }
}

fn create_versions(version: &str) -> LoadVersionsOutput {
    LoadVersionsOutput {
        versions: vec![VersionSpec::parse(version).unwrap()],
        ..Default::default()
    }
}

mod inventory {
    use super::*;

    mod remote_versions_cache {
        use super::*;

        #[test]
        fn saves_and_loads_without_scope() {
            let sandbox = create_empty_sandbox();
            let inventory = create_inventory(sandbox.path());

            inventory
                .save_remote_versions(&create_versions("1.2.3"), None)
                .unwrap();

            assert!(sandbox.path().join("remote-versions.json").exists());

            let data = inventory
                .load_remote_versions(false, None)
                .unwrap()
                .unwrap();

            assert_eq!(data.versions, vec![VersionSpec::parse("1.2.3").unwrap()]);
        }

        #[test]
        fn saves_and_loads_with_scope() {
            let sandbox = create_empty_sandbox();
            let inventory = create_inventory(sandbox.path());

            inventory
                .save_remote_versions(&create_versions("1.2.3"), Some("temurin"))
                .unwrap();

            assert!(sandbox.path().join("remote-versions-temurin.json").exists());
            assert!(!sandbox.path().join("remote-versions.json").exists());

            let data = inventory
                .load_remote_versions(false, Some("temurin"))
                .unwrap()
                .unwrap();

            assert_eq!(data.versions, vec![VersionSpec::parse("1.2.3").unwrap()]);
        }

        #[test]
        fn isolates_scopes_from_each_other() {
            let sandbox = create_empty_sandbox();
            let inventory = create_inventory(sandbox.path());

            inventory
                .save_remote_versions(&create_versions("1.0.0"), None)
                .unwrap();
            inventory
                .save_remote_versions(&create_versions("2.0.0"), Some("temurin"))
                .unwrap();
            inventory
                .save_remote_versions(&create_versions("3.0.0"), Some("zulu"))
                .unwrap();

            for (scope, version) in [
                (None, "1.0.0"),
                (Some("temurin"), "2.0.0"),
                (Some("zulu"), "3.0.0"),
            ] {
                let data = inventory
                    .load_remote_versions(false, scope)
                    .unwrap()
                    .unwrap();

                assert_eq!(
                    data.versions,
                    vec![VersionSpec::parse(version).unwrap()],
                    "scope: {scope:?}"
                );
            }

            assert!(
                inventory
                    .load_remote_versions(false, Some("corretto"))
                    .unwrap()
                    .is_none()
            );
        }

        #[test]
        fn returns_none_when_missing() {
            let sandbox = create_empty_sandbox();
            let inventory = create_inventory(sandbox.path());

            assert!(
                inventory
                    .load_remote_versions(false, None)
                    .unwrap()
                    .is_none()
            );
            assert!(
                inventory
                    .load_remote_versions(false, Some("temurin"))
                    .unwrap()
                    .is_none()
            );
        }

        #[test]
        fn returns_none_when_cache_disabled() {
            let sandbox = create_empty_sandbox();
            let inventory = create_inventory(sandbox.path());

            inventory
                .save_remote_versions(&create_versions("1.2.3"), Some("temurin"))
                .unwrap();

            assert!(
                inventory
                    .load_remote_versions(true, Some("temurin"))
                    .unwrap()
                    .is_none()
            );
        }

        #[test]
        fn encodes_scope_into_file_name() {
            let sandbox = create_empty_sandbox();
            let inventory = create_inventory(sandbox.path());

            inventory
                .save_remote_versions(&create_versions("1.2.3"), Some("scope/with:chars"))
                .unwrap();

            // The scope must not create nested directories
            for entry in std::fs::read_dir(sandbox.path()).unwrap() {
                assert!(entry.unwrap().file_type().unwrap().is_file());
            }

            let data = inventory
                .load_remote_versions(false, Some("scope/with:chars"))
                .unwrap()
                .unwrap();

            assert_eq!(data.versions, vec![VersionSpec::parse("1.2.3").unwrap()]);
        }

        #[test]
        fn writes_to_original_dir_when_set() {
            let sandbox = create_empty_sandbox();
            let original_dir = sandbox.path().join("original");
            let mut inventory = create_inventory(&sandbox.path().join("current"));
            inventory.dir_original = Some(original_dir.clone());

            inventory
                .save_remote_versions(&create_versions("1.2.3"), Some("temurin"))
                .unwrap();

            assert!(original_dir.join("remote-versions-temurin.json").exists());
            assert!(
                inventory
                    .load_remote_versions(false, Some("temurin"))
                    .unwrap()
                    .is_some()
            );
        }
    }
}
