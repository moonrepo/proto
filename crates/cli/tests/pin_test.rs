mod utils;

use proto_core::{UnresolvedVersionSpec, UserConfig};
use std::fs;
use utils::*;

mod pin_local {
    use super::*;

    #[test]
    fn writes_local_version_file() {
        let sandbox = create_empty_sandbox();
        let version_file = sandbox.path().join(".prototools");

        assert!(!version_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("pin").arg("node").arg("19.0.0").assert().success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "node = \"19.0.0\"\n"
        )
    }

    #[test]
    fn appends_multiple_tools() {
        let sandbox = create_empty_sandbox();
        let version_file = sandbox.path().join(".prototools");

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("pin").arg("node").arg("19.0.0").assert().success();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("pin").arg("npm").arg("9.0.0").assert().success();

        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            r#"node = "19.0.0"
npm = "9.0.0"
"#
        )
    }

    #[test]
    fn will_overwrite_by_name() {
        let sandbox = create_empty_sandbox();
        let version_file = sandbox.path().join(".prototools");

        sandbox.create_file(
            ".prototools",
            r#"node = "16.0.0"
npm = "9.0.0"
"#,
        );

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("pin").arg("node").arg("19").assert().success();

        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            r#"node = "~19"
npm = "9.0.0"
"#
        )
    }

    #[test]
    fn can_set_aliases() {
        let sandbox = create_empty_sandbox();
        let version_file = sandbox.path().join(".prototools");

        assert!(!version_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("pin").arg("npm").arg("bundled").assert().success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "npm = \"bundled\"\n"
        )
    }

    #[test]
    fn can_set_partial_version() {
        let sandbox = create_empty_sandbox();
        let version_file = sandbox.path().join(".prototools");

        assert!(!version_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("pin").arg("npm").arg("1.2").assert().success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "npm = \"~1.2\"\n"
        )
    }
}

mod pin_global {
    use super::*;

    #[test]
    fn updates_manifest_file() {
        let sandbox = create_empty_sandbox();
        let config_file = sandbox.path().join("config.toml");

        assert!(!config_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("pin")
            .arg("--global")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        assert!(config_file.exists());

        let config = UserConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            config.tools.get("node").unwrap().default_version,
            Some(UnresolvedVersionSpec::parse("19.0.0").unwrap())
        );
    }

    #[test]
    fn can_set_alias_as_default() {
        let sandbox = create_empty_sandbox();
        let config_file = sandbox.path().join("config.toml");

        assert!(!config_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("pin")
            .arg("--global")
            .arg("npm")
            .arg("bundled")
            .assert()
            .success();

        assert!(config_file.exists());

        let config = UserConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            config.tools.get("npm").unwrap().default_version,
            Some(UnresolvedVersionSpec::Alias("bundled".into()))
        );
    }

    #[test]
    fn can_set_partial_version_as_default() {
        let sandbox = create_empty_sandbox();
        let config_file = sandbox.path().join("config.toml");

        assert!(!config_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("pin")
            .arg("--global")
            .arg("npm")
            .arg("1.2")
            .assert()
            .success();

        assert!(config_file.exists());

        let config = UserConfig::load_from(sandbox.path()).unwrap();

        assert_eq!(
            config.tools.get("npm").unwrap().default_version,
            Some(UnresolvedVersionSpec::parse("1.2").unwrap())
        );
    }

    #[test]
    fn doesnt_create_bin_symlink() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("pin")
            .arg("--global")
            .arg("node")
            .arg("20")
            .assert()
            .success();

        let link = sandbox
            .path()
            .join("bin")
            .join(if cfg!(windows) { "node.exe" } else { "node" });

        assert!(!link.exists());
    }
}
