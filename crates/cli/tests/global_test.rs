mod utils;

use proto_core::{ToolManifest, VersionType};
use utils::*;

mod global {
    use super::*;

    #[test]
    fn updates_manifest_file() {
        let temp = create_empty_sandbox();
        let manifest_file = temp.path().join("tools/node/manifest.json");

        assert!(!manifest_file.exists());

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("global")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        assert!(manifest_file.exists());

        let manifest = ToolManifest::load(manifest_file).unwrap();

        assert_eq!(
            manifest.default_version,
            Some(VersionType::parse("19.0.0").unwrap())
        );
    }

    #[test]
    fn can_set_alias_as_default() {
        let temp = create_empty_sandbox();
        let manifest_file = temp.path().join("tools/npm/manifest.json");

        assert!(!manifest_file.exists());

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("global")
            .arg("npm")
            .arg("bundled")
            .assert()
            .success();

        assert!(manifest_file.exists());

        let manifest = ToolManifest::load(manifest_file).unwrap();

        assert_eq!(
            manifest.default_version,
            Some(VersionType::Alias("bundled".into()))
        );
    }
}
