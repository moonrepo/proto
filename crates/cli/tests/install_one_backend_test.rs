mod utils;

#[cfg(not(windows))]
mod install_one_backend {
    use super::utils::*;
    use proto_core::{ToolContext, ToolSpec, UnresolvedVersionSpec};
    use starbase_sandbox::predicates::prelude::*;

    #[test]
    fn installs_and_uninstalls_asdf_tool() {
        let sandbox = create_empty_proto_sandbox();
        let tool_dir = sandbox.path().join(".proto/tools/zig/0.13.0");

        assert!(!tool_dir.exists());

        // Install
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("asdf:zig").arg("0.13.0");
            })
            .success();

        assert!(tool_dir.exists());

        assert.stdout(predicate::str::contains(
            "asdf:zig 0.13.0 has been installed",
        ));

        // Uninstall
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("asdf:zig")
                    .arg("0.13.0")
                    .arg("--yes");
            })
            .success();

        assert!(!tool_dir.exists());

        assert.stdout(predicate::str::contains(
            "asdf:zig 0.13.0 has been uninstalled!",
        ));
    }

    #[test]
    fn installs_and_pins_backend() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install")
                    .arg("asdf:zig")
                    .arg("0.13.0")
                    .arg("--pin")
                    .arg("local");
            })
            .success();

        let config = load_config(sandbox.path());

        assert_eq!(
            config
                .versions
                .get(&ToolContext::parse("asdf:zig").unwrap())
                .unwrap(),
            &ToolSpec::new(UnresolvedVersionSpec::parse("0.13.0").unwrap(),)
        );
    }

    #[test]
    fn installs_with_shortname() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
[tools.newrelic]
asdf-shortname = "newrelic-cli"
"#,
        );

        let tool_dir = sandbox.path().join(".proto/tools/newrelic/0.97.0");

        assert!(!tool_dir.exists());

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("asdf:newrelic").arg("0.97.0");
            })
            .success();

        assert!(tool_dir.exists());

        assert.stdout(predicate::str::contains(
            "asdf:newrelic 0.97.0 has been installed",
        ));
    }

    #[test]
    fn installs_with_custom_repo() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
[tools.newrelic]
asdf-repository = "https://github.com/NeoHsu/asdf-newrelic-cli"
"#,
        );

        let tool_dir = sandbox.path().join(".proto/tools/newrelic/0.97.0");

        assert!(!tool_dir.exists());

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("asdf:newrelic").arg("0.97.0");
            })
            .success();

        assert!(tool_dir.exists());

        assert.stdout(predicate::str::contains(
            "asdf:newrelic 0.97.0 has been installed",
        ));
    }
}
