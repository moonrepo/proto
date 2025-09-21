mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod exec {
    use super::*;

    #[test]
    fn errors_if_no_command() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("exec");
        });

        assert.inner.stderr(predicate::str::contains(
            "A command is required for execution.",
        ));
    }

    #[test]
    fn errors_for_invalid_context() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "foo bar", "--", "echo"]);
        });

        assert.inner.stderr(predicate::str::contains(
            "Invalid identifier format for foo bar.",
        ));
    }

    #[test]
    fn errors_for_invalid_spec() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "tool@a b c", "--", "echo"]);
        });

        assert.inner.stderr(predicate::str::contains(
            "Invalid version or requirement in tool specification a b c.",
        ));
    }

    #[test]
    fn can_execute_without_tools() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "--", "echo", "hello"]);
        });

        assert.inner.stdout(predicate::str::contains("hello"));
    }

    #[test]
    fn one_tool() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.args(["install", "node", "20"]);
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "node", "--", "node", "--version"]);
        });

        assert.inner.stdout(predicate::str::contains("v20.19.5"));
    }

    #[test]
    fn many_tools() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
node = "20"
bun = "1.2"
"#,
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args([
                "exec",
                "node",
                "bun",
                "--shell",
                "bash",
                "--raw",
                "--",
                "node --version && bun --version",
            ]);
        });

        assert
            .inner
            .stdout(predicate::str::contains("v20.19.5").and(predicate::str::contains("1.2.22")));
    }

    #[test]
    fn can_use_all_config_tools() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".prototools",
            r#"
node = "20"
bun = "1.2"
"#,
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install");
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args([
                "exec",
                "--tools-from-config",
                "--shell",
                "bash",
                "--raw",
                "--",
                "node --version && bun --version",
            ]);
        });

        assert
            .inner
            .stdout(predicate::str::contains("v20.19.5").and(predicate::str::contains("1.2.22")));
    }

    #[test]
    fn can_scope_by_version_part() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.args(["install", "node", "20.19.5"]);
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.args(["install", "node", "20.18.3"]);
            })
            .success();

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "node@20", "--", "node", "--version"]);
        });

        assert.inner.stdout(predicate::str::contains("v20.19.5"));

        let assert = sandbox.run_bin(|cmd| {
            cmd.args(["exec", "node@20.18", "--", "node", "--version"]);
        });

        assert.inner.stdout(predicate::str::contains("v20.18.3"));
    }
}
