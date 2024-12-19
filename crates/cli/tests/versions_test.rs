mod utils;

use proto_core::{ToolManifest, ToolManifestVersion, VersionSpec};
use starbase_sandbox::output_to_string;
use utils::*;

mod versions {
    use super::*;

    #[test]
    fn lists_remote_versions() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("npm");
        });

        // Without stderr
        let output = output_to_string(&assert.inner.get_output().stdout);

        assert!(output.split('\n').collect::<Vec<_>>().len() > 1);
    }

    #[test]
    fn lists_local_versions() {
        let sandbox = create_empty_proto_sandbox();
        let versions = vec!["19.0.0", "18.0.0", "17.0.0"];

        let mut manifest =
            ToolManifest::load(sandbox.path().join(".proto/tools/node/manifest.json")).unwrap();

        for version in &versions {
            manifest.versions.insert(
                VersionSpec::parse(version).unwrap(),
                ToolManifestVersion::default(),
            );
        }

        manifest.save().unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("node");
        });

        // Without stderr
        let output = output_to_string(&assert.inner.get_output().stdout);
        let mut count = 0;

        for line in output.lines() {
            for version in &versions {
                if line.starts_with(version) {
                    count += 1;
                    assert!(line.contains("installed"));
                }
            }
        }

        assert_eq!(count, 3);
    }

    #[test]
    fn only_displays_local_versions() {
        let sandbox = create_empty_proto_sandbox();
        let versions = vec!["19.0.0", "18.0.0", "17.0.0"];

        let mut manifest =
            ToolManifest::load(sandbox.path().join(".proto/tools/node/manifest.json")).unwrap();

        for version in &versions {
            manifest.versions.insert(
                VersionSpec::parse(version).unwrap(),
                ToolManifestVersion::default(),
            );
        }

        manifest.save().unwrap();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("versions").arg("node").arg("--installed");
        });

        assert.debug();

        // Without stderr
        let output = output_to_string(&assert.inner.get_output().stdout);

        assert_eq!(output.lines().collect::<Vec<_>>().len(), 3);
    }
}
