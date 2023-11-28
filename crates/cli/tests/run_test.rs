mod utils;

use proto_core::{ToolManifest, VersionSpec};
use starbase_sandbox::predicates::prelude::*;
use std::{env, fs};
use utils::*;

mod run {
    use super::*;

    #[test]
    fn errors_if_not_installed() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("run").arg("node").arg("19.0.0").assert();

        assert.stderr(predicate::str::contains(
            "This project requires Node.js 19.0.0",
        ));
    }

    #[test]
    fn errors_if_no_version_detected() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("run").arg("node").assert();

        assert.stderr(predicate::str::contains(
            "Failed to detect an applicable version",
        ));
    }

    #[test]
    fn runs_a_tool() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("19.0.0"));
    }

    #[test]
    fn runs_a_tool_using_version_detection() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        // Arg
        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("19.0.0"));

        // Env var
        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .env("PROTO_NODE_VERSION", "19.0.0")
            .arg("run")
            .arg("node")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("19.0.0"));

        // Local version
        temp.create_file(".prototools", "node = \"19.0.0\"");

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("19.0.0"));

        fs::remove_file(temp.path().join(".prototools")).unwrap();

        // Global version
        temp.create_file("config.toml", "[tools.node]\ndefault-version = \"19.0.0\"");

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("19.0.0"));

        fs::remove_file(temp.path().join("config.toml")).unwrap();
    }

    // This test fails in Windows for some reason, but works fine with `cargo run`...
    #[cfg(not(windows))]
    #[test]
    fn runs_a_tool_alt_bin() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--bin")
            .arg(if cfg!(windows) { "npx.cmd" } else { "bin/npx" })
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("8.19.2"));
    }

    #[test]
    fn updates_last_used_at() {
        let temp = create_empty_sandbox();
        let manifest_file = temp.path().join("tools/node/manifest.json");

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        let mut cmd = create_proto_command(temp.path());
        cmd.arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        let manifest = ToolManifest::load(&manifest_file).unwrap();
        let version = VersionSpec::parse("19.0.0").unwrap();

        let last_used_at = manifest.versions.get(&version).unwrap().last_used_at;

        assert!(last_used_at.is_some());

        // Run again and make sure timestamps update
        let mut cmd = create_proto_command(temp.path());
        cmd.arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        let manifest = ToolManifest::load(&manifest_file).unwrap();

        assert_ne!(
            last_used_at,
            manifest.versions.get(&version).unwrap().last_used_at
        );
    }

    #[test]
    fn auto_installs_if_missing() {
        let temp = create_empty_sandbox();

        temp.create_file("config.toml", "auto-install = true");

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("19.0.0"));
    }

    #[test]
    fn auto_installs_if_missing_with_env_var() {
        let temp = create_empty_sandbox();

        env::set_var("PROTO_AUTO_INSTALL", "true");

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("19.0.0"));

        env::remove_var("PROTO_AUTO_INSTALL");
    }

    #[test]
    fn doesnt_auto_install_if_false() {
        let temp = create_empty_sandbox();

        temp.create_file("config.toml", "auto-install = false");

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("run").arg("node").arg("19.0.0").assert();

        assert.stderr(predicate::str::contains(
            "This project requires Node.js 19.0.0",
        ));
    }

    #[test]
    fn doesnt_auto_install_subsequently() {
        let temp = create_empty_sandbox();

        temp.create_file("config.toml", "auto-install = true");

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stderr(predicate::str::contains("Node.js has been installed"));

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stderr(predicate::str::contains("Node.js has been installed").not());
    }

    #[test]
    fn errors_if_plugin_not_configured() {
        let temp = create_empty_sandbox();

        let mut cmd = create_proto_command(temp.path());
        let assert = cmd.arg("run").arg("plugin-name").arg("1.0.0").assert();

        assert.stderr(predicate::str::contains(
            "plugin-name is not a built-in tool or has not been configured as a plugin",
        ));
    }
}
