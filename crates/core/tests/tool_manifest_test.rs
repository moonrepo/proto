use proto_core::{ToolManifest, ToolManifestVersion};
use starbase_sandbox::create_empty_sandbox;
use version_spec::VersionSpec;

mod tool_manifest {
    use super::*;

    mod reload_from_disk {
        use super::*;

        #[test]
        fn does_nothing_if_no_file() {
            let sandbox = create_empty_sandbox();
            let mut manifest = ToolManifest::load_from(sandbox.path()).unwrap();

            manifest.reload_from_disk().unwrap();

            assert!(manifest.installed_versions.is_empty());
        }

        #[test]
        fn merges_versions_saved_by_another_process() {
            let sandbox = create_empty_sandbox();

            // Simulates 2 processes loading the same manifest
            let mut ours = ToolManifest::load_from(sandbox.path()).unwrap();
            let mut theirs = ToolManifest::load_from(sandbox.path()).unwrap();

            let our_version = VersionSpec::parse("2.0.0").unwrap();
            let their_version = VersionSpec::parse("1.0.0").unwrap();

            theirs.add_version(&their_version, ToolManifestVersion::default());
            theirs.save().unwrap();

            ours.add_version(&our_version, ToolManifestVersion::default());
            ours.reload_from_disk().unwrap();

            // Both the version from disk and the unsaved in-memory
            // version are present
            assert!(ours.installed_versions.contains(&their_version));
            assert!(ours.installed_versions.contains(&our_version));
            assert!(ours.versions.contains_key(&their_version));
            assert!(ours.versions.contains_key(&our_version));

            // And both persist after saving
            ours.save().unwrap();

            let reloaded = ToolManifest::load_from(sandbox.path()).unwrap();

            assert!(reloaded.installed_versions.contains(&their_version));
            assert!(reloaded.installed_versions.contains(&our_version));
        }

        #[test]
        fn keeps_in_memory_entry_on_conflict() {
            let sandbox = create_empty_sandbox();

            let mut ours = ToolManifest::load_from(sandbox.path()).unwrap();
            let mut theirs = ToolManifest::load_from(sandbox.path()).unwrap();

            let version = VersionSpec::parse("1.0.0").unwrap();

            theirs.add_version(
                &version,
                ToolManifestVersion {
                    no_clean: false,
                    ..Default::default()
                },
            );
            theirs.save().unwrap();

            ours.add_version(
                &version,
                ToolManifestVersion {
                    no_clean: true,
                    ..Default::default()
                },
            );
            ours.reload_from_disk().unwrap();

            assert!(ours.versions.get(&version).unwrap().no_clean);
        }

        #[test]
        fn takes_the_highest_shim_version() {
            let sandbox = create_empty_sandbox();

            let mut ours = ToolManifest::load_from(sandbox.path()).unwrap();
            let mut theirs = ToolManifest::load_from(sandbox.path()).unwrap();

            theirs.shim_version = 5;
            theirs.save().unwrap();

            ours.shim_version = 3;
            ours.reload_from_disk().unwrap();

            assert_eq!(ours.shim_version, 5);
        }
    }
}
