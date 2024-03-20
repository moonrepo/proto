mod utils;

use starbase_sandbox::{
    assert_snapshot, create_sandbox, get_assert_output, predicates::prelude::*,
};
use std::path::Path;
use std::{env, fs};
use utils::*;

fn install_node(sandbox: &Path) {
    let mut cmd = create_proto_command(sandbox);
    cmd.arg("install")
        .arg("node")
        .arg("19.0.0")
        .assert()
        .success();
}

mod run {
    use super::*;

    #[test]
    fn errors_if_not_installed() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("run").arg("node").arg("19.0.0").assert();

        assert.stderr(predicate::str::contains(
            "This project requires Node.js 19.0.0",
        ));
    }

    #[test]
    fn errors_if_no_version_detected() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("run").arg("node").assert();

        assert.stderr(predicate::str::contains(
            "Failed to detect an applicable version",
        ));
    }

    #[test]
    fn runs_a_tool() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        let mut cmd = create_proto_command(sandbox.path());
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
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        // Arg
        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("19.0.0"));

        // Env var
        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .env("PROTO_NODE_VERSION", "19.0.0")
            .arg("run")
            .arg("node")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("19.0.0"));

        // Local version
        sandbox.create_file(".prototools", "node = \"19.0.0\"");

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("19.0.0"));

        fs::remove_file(sandbox.path().join(".prototools")).unwrap();

        // Global version
        sandbox.create_file(".proto/.prototools", "node = \"19.0.0\"");

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("19.0.0"));
    }

    #[test]
    fn updates_last_used_at() {
        let sandbox = create_empty_sandbox();
        let last_used_file = sandbox.path().join(".proto/tools/node/19.0.0/.last-used");

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("install")
            .arg("node")
            .arg("19.0.0")
            .assert()
            .success();

        assert!(!last_used_file.exists());

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        let value = fs::read_to_string(&last_used_file).unwrap();

        assert!(last_used_file.exists());
        assert_ne!(value, "");

        // Run again and make sure timestamps update
        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        let new_value = fs::read_to_string(&last_used_file).unwrap();

        assert!(last_used_file.exists());
        assert_ne!(value, new_value);
    }

    #[test]
    fn auto_installs_if_missing() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(".prototools", "[settings]\nauto-install = true");

        let mut cmd = create_proto_command(sandbox.path());
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
        let sandbox = create_empty_sandbox();

        env::set_var("PROTO_AUTO_INSTALL", "true");

        let mut cmd = create_proto_command(sandbox.path());
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
        let sandbox = create_empty_sandbox();

        sandbox.create_file(".prototools", "[settings]\nauto-install = false");

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("run").arg("node").arg("19.0.0").assert();

        assert.stderr(predicate::str::contains(
            "This project requires Node.js 19.0.0",
        ));
    }

    #[test]
    fn doesnt_auto_install_subsequently() {
        let sandbox = create_empty_sandbox();

        sandbox.create_file(".prototools", "[settings]\nauto-install = true");

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains(
            "Node.js 19.0.0 has been installed",
        ));

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd
            .arg("run")
            .arg("node")
            .arg("19.0.0")
            .arg("--")
            .arg("--version")
            .assert();

        assert.stdout(predicate::str::contains("Node.js 19.0.0 has been installed").not());
    }

    #[test]
    fn errors_if_plugin_not_configured() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        let assert = cmd.arg("run").arg("plugin-name").arg("1.0.0").assert();

        assert.stderr(predicate::str::contains(
            "plugin-name is not a built-in tool or has not been configured as a plugin",
        ));
    }

    mod env_vars {
        use super::*;

        #[test]
        fn inherits_from_config() {
            let sandbox = create_sandbox("env-vars");

            sandbox.create_file(
                ".prototools",
                r#"
[tools.node.env]
FROM_CONFIG = "abc123"
FROM_CONFIG_BOOL = true
"#,
            );

            install_node(sandbox.path());

            let mut cmd = create_proto_command(sandbox.path());
            let assert = cmd
                .arg("run")
                .arg("node")
                .arg("19.0.0")
                .arg("--")
                .arg("test.js")
                .assert();

            assert_snapshot!(get_assert_output(&assert));
        }

        #[test]
        fn inherits_from_parent() {
            let sandbox = create_sandbox("env-vars");

            install_node(sandbox.path());

            let mut cmd = create_proto_command(sandbox.path());
            let assert = cmd
                .arg("run")
                .arg("node")
                .arg("19.0.0")
                .arg("--")
                .arg("test.js")
                .env("FROM_PARENT", "abc123")
                .assert();

            assert_snapshot!(get_assert_output(&assert));
        }

        #[test]
        fn can_disable_inherits_from_parent_with_config() {
            let sandbox = create_sandbox("env-vars");

            sandbox.create_file(
                ".prototools",
                r#"
[tools.node.env]
FROM_PARENT_REMOVED = false
"#,
            );

            install_node(sandbox.path());

            let mut cmd = create_proto_command(sandbox.path());
            let assert = cmd
                .arg("run")
                .arg("node")
                .arg("19.0.0")
                .arg("--")
                .arg("test.js")
                .env("FROM_PARENT", "abc123")
                .env("FROM_PARENT_REMOVED", "abc123")
                .assert();

            assert_snapshot!(get_assert_output(&assert));
        }

        #[test]
        fn parent_overrides_config() {
            let sandbox = create_sandbox("env-vars");

            sandbox.create_file(
                ".prototools",
                r#"
[tools.node.env]
FROM_CONFIG = "abc123"
"#,
            );

            install_node(sandbox.path());

            let mut cmd = create_proto_command(sandbox.path());
            let assert = cmd
                .arg("run")
                .arg("node")
                .arg("19.0.0")
                .arg("--")
                .arg("test.js")
                .env("FROM_CONFIG", "xyz789")
                .env("FROM_PARENT", "xyz789")
                .assert();

            assert_snapshot!(get_assert_output(&assert));
        }

        #[test]
        fn supports_interpolation() {
            let sandbox = create_sandbox("env-vars");

            sandbox.create_file(
                ".prototools",
                r#"
[tools.node.env]
FIRST = "abc"
SECOND = "123"
THIRD = "value-${FIRST}-${SECOND}-${PARENT}"
FOURTH = "ignores-$FIRST-$PARENT"
"#,
            );

            install_node(sandbox.path());

            let mut cmd = create_proto_command(sandbox.path());
            let assert = cmd
                .arg("run")
                .arg("node")
                .arg("19.0.0")
                .arg("--")
                .arg("interpolation.js")
                .env("SECOND", "789")
                .env("PARENT", "xyz")
                .assert();

            assert_snapshot!(get_assert_output(&assert));
        }
    }
}
