mod utils;

use proto_core::{ToolManifest, ToolsConfig, UnresolvedVersionSpec, VersionSpec};
use starbase_sandbox::predicates::prelude::*;
use std::collections::HashSet;
use utils::*;

mod install_uninstall {
    use super::*;

    #[test]
    fn installs_without_patch() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("18.12")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        assert!(temp.path().join("tools/node/18.12.1").exists());
    }

    #[test]
    fn installs_without_minor() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("17")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        assert!(temp.path().join("tools/node/17.9.1").exists());
    }

    #[test]
    fn installs_from_alias() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("gallium")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        assert!(temp.path().join("tools/node/16.20.2").exists());
    }

    #[test]
    fn installs_and_uninstalls_tool() {
        let temp = create_empty_sandbox();
        let tool_dir = temp.path().join("tools/node/19.0.0");

        assert!(!tool_dir.exists());

        // Install
        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        assert!(tool_dir.exists());

        assert.stderr(predicate::str::contains("Node.js has been installed"));

        // Uninstall
        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("uninstall").arg("node").arg("19.0.0").assert();

        assert!(!tool_dir.exists());

        assert.stderr(predicate::str::contains(
            "Node.js 19.0.0 has been uninstalled!",
        ));
    }

    #[test]
    fn doesnt_install_tool_if_exists() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        assert.stderr(predicate::str::contains(
            "Node.js has already been installed",
        ));
    }

    #[test]
    fn updates_the_manifest_when_installing() {
        let temp = create_empty_sandbox();
        let manifest_file = temp.path().join("tools/node/manifest.json");

        // Install
        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert()
            .success();

        let manifest = ToolManifest::load(&manifest_file).unwrap();

        assert_eq!(
            manifest.default_version,
            Some(UnresolvedVersionSpec::parse("19.0.0").unwrap())
        );
        assert_eq!(
            manifest.installed_versions,
            HashSet::from_iter([VersionSpec::parse("19.0.0").unwrap()])
        );
        assert!(manifest
            .versions
            .contains_key(&VersionSpec::parse("19.0.0").unwrap()));

        // Uninstall
        let mut cmd = create_proto_command(temp.path());
        cmd.arg("uninstall")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        let manifest = ToolManifest::load(&manifest_file).unwrap();

        assert_eq!(manifest.default_version, None);
        assert_eq!(manifest.installed_versions, HashSet::default());
        assert!(!manifest
            .versions
            .contains_key(&VersionSpec::parse("19.0.0").unwrap()));
    }

    #[test]
    fn can_pin_when_installing() {
        let temp = create_empty_sandbox();
        let manifest_file = temp.path().join("tools/node/manifest.json");

        let mut manifest = ToolManifest::load(&manifest_file).unwrap();
        manifest.default_version = Some(UnresolvedVersionSpec::parse("18.0.0").unwrap());
        manifest
            .installed_versions
            .insert(VersionSpec::parse("18.0.0").unwrap());
        manifest.save().unwrap();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--pin")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        let manifest = ToolManifest::load(&manifest_file).unwrap();

        assert_eq!(
            manifest.default_version,
            Some(UnresolvedVersionSpec::parse("19.0.0").unwrap())
        );
        assert_eq!(
            manifest.installed_versions,
            HashSet::from_iter([
                VersionSpec::parse("18.0.0").unwrap(),
                VersionSpec::parse("19.0.0").unwrap(),
            ])
        );
    }

    #[test]
    fn can_pin_when_already_installed() {
        let temp = create_empty_sandbox();
        let manifest_file = temp.path().join("tools/node/manifest.json");

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        // Manually change it to something else
        let mut manifest = ToolManifest::load(&manifest_file).unwrap();
        manifest.default_version = Some(UnresolvedVersionSpec::parse("18.0.0").unwrap());
        manifest.save().unwrap();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--pin")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        let manifest = ToolManifest::load(&manifest_file).unwrap();

        assert_eq!(
            manifest.default_version,
            Some(UnresolvedVersionSpec::parse("19.0.0").unwrap())
        );
    }

    #[test]
    fn can_pin_latest_locally_using_setting() {
        let temp = create_empty_sandbox();
        temp.create_file("config.toml", "pin-latest = \"local\"");

        let manifest_file = temp.path().join("tools/node/manifest.json");

        // We set a default version here to compare against later
        let mut manifest = ToolManifest::load(&manifest_file).unwrap();
        manifest.default_version = Some(UnresolvedVersionSpec::parse("18.0.0").unwrap());
        manifest.save().unwrap();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        let manifest = ToolManifest::load(&manifest_file).unwrap();

        assert_eq!(
            manifest.default_version.unwrap(),
            UnresolvedVersionSpec::parse("18.0.0").unwrap()
        );

        let tools = ToolsConfig::load_from(temp.path()).unwrap();

        assert!(tools.tools.get("node").is_some());
    }

    #[test]
    fn can_pin_latest_globally_using_setting() {
        let temp = create_empty_sandbox();
        temp.create_file("config.toml", "pin-latest = \"global\"");

        let manifest_file = temp.path().join("tools/node/manifest.json");

        // We set a default version here to compare against later
        let mut manifest = ToolManifest::load(&manifest_file).unwrap();
        manifest.default_version = Some(UnresolvedVersionSpec::parse("18.0.0").unwrap());
        manifest.save().unwrap();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        let manifest = ToolManifest::load(&manifest_file).unwrap();

        assert_ne!(
            manifest.default_version.unwrap(),
            UnresolvedVersionSpec::parse("18.0.0").unwrap()
        );

        let tools = ToolsConfig::load_from(temp.path()).unwrap();

        assert!(tools.tools.get("node").is_none());
    }

    #[test]
    fn doesnt_pin_using_setting_if_not_latest() {
        let temp = create_empty_sandbox();
        temp.create_file("config.toml", "pin-latest = \"local\"");

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("20.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        let tools = ToolsConfig::load_from(temp.path()).unwrap();

        assert!(tools.tools.get("node").is_none());
    }

    #[test]
    fn symlinks_bin_when_pinning() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--pin")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        let link = temp
            .path()
            .join("bin")
            .join(if cfg!(windows) { "node.exe" } else { "node" });

        assert!(link.exists());

        assert_eq!(
            std::fs::read_link(link).unwrap(),
            temp.path()
                .join("tools/node/19.0.0")
                .join(if cfg!(windows) {
                    "node.exe"
                } else {
                    "bin/node"
                })
        );
    }

    #[test]
    fn symlinks_bin_on_first_install_without_pinning() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--no-bundled-npm")
            .assert();

        let link = temp
            .path()
            .join("bin")
            .join(if cfg!(windows) { "node.exe" } else { "node" });

        assert!(link.exists());
    }
}
