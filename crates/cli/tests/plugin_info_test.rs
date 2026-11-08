use proto_core::test_utils::*;
use starbase_sandbox::predicates::prelude::*;

mod plugin_info {
    use super::*;

    // Windows doesn't support asdf
    #[cfg(not(windows))]
    #[test]
    fn supports_tools_with_a_backend() {
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

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("plugin").arg("info").arg("asdf:zig");
        });

        assert.success().stdout(
            predicate::str::contains("asdf:zig")
                .and(predicate::str::contains("Backend"))
                .and(predicate::str::contains("Inventory"))
                .and(predicate::str::contains("0.13.0")),
        );

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("plugin").arg("info").arg("asdf:zig").arg("--json");
        });
        let stdout = assert.stdout();
        assert.success();
        let output: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        assert_eq!(output["context"], "asdf:zig");
        assert_eq!(output["id"], "zig");
        assert!(
            output["exe_file"]
                .as_str()
                .is_some_and(|file| file.contains("zig/0.13.0"))
        );
    }
}
