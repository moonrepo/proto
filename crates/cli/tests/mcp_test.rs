mod mcp {
    use proto_core::test_utils::*;
    use proto_core::{ProtoLock, ToolManifest};
    use serde_json::{Value, json};
    use starbase_sandbox::Sandbox;
    use std::fs;
    use system_env::{SystemArch, SystemOS};

    fn call_tool(sandbox: &Sandbox, name: &str, arguments: Value) -> Value {
        let input = [
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "0.0.0" }
                }
            }),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": { "name": name, "arguments": arguments }
            }),
        ]
        .iter()
        .map(|request| format!("{request}\n"))
        .collect::<String>();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("mcp").write_stdin(input);
        });
        let stdout = assert.stdout();

        let response = stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find(|record| record.get("id") == Some(&json!(2)))
            .unwrap_or_else(|| panic!("missing tools/call response in stdout: {stdout}"));

        assert.success();

        let result = response
            .get("result")
            .unwrap_or_else(|| panic!("tools/call failed: {response}"));

        assert_ne!(
            result.get("isError"),
            Some(&json!(true)),
            "tool returned an error: {result}"
        );

        result["structuredContent"].clone()
    }

    fn load_manifest(sandbox: &Sandbox) -> ToolManifest {
        ToolManifest::load(sandbox.path().join(".proto/tools/protostar/manifest.json")).unwrap()
    }

    #[test]
    fn stdout_only_carries_jsonrpc_in_agent_environments() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("mcp")
                .env("CODEX_CI", "1")
                .env_remove("PROTO_REPORTER")
                .write_stdin(
                    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#
                        .to_owned()
                        + "\n",
                );
        });
        let stdout = assert.stdout();

        assert!(stdout.contains("\"jsonrpc\":\"2.0\""));

        for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
            let record: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|error| {
                panic!("stdout line is not valid JSON-RPC: {line}: {error}")
            });

            assert_eq!(
                record.get("jsonrpc").and_then(|value| value.as_str()),
                Some("2.0"),
                "stdout line is not a JSON-RPC record: {line}",
            );
        }

        assert.success();
    }

    mod uninstall_tool {
        use super::*;

        #[test]
        fn removes_version_from_manifest() {
            let sandbox = create_empty_proto_sandbox();

            for version in ["1.0.0", "2.0.0"] {
                sandbox
                    .run_bin(|cmd| {
                        cmd.arg("install").arg("protostar").arg(version);
                    })
                    .success();
            }

            let result = call_tool(
                &sandbox,
                "uninstall_tool",
                json!({ "tool": "protostar", "spec": "1.0.0" }),
            );

            assert_eq!(result, json!({ "uninstalled": true, "spec": "1.0.0" }));
            assert!(!sandbox.path().join(".proto/tools/protostar/1.0.0").exists());

            let manifest = load_manifest(&sandbox);

            assert_eq!(
                manifest
                    .installed_versions
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
                ["2.0.0"]
            );
            assert_eq!(
                manifest
                    .versions
                    .keys()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
                ["2.0.0"]
            );
        }

        #[test]
        fn removes_bins_and_shims_for_last_version() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("1.0.0");
                })
                .success();

            let bin = sandbox.path().join(".proto/bin").join(if cfg!(windows) {
                "protostar.exe"
            } else {
                "protostar"
            });
            let shim = sandbox.path().join(".proto/shims").join(if cfg!(windows) {
                "protostar.exe"
            } else {
                "protostar"
            });

            assert!(bin.symlink_metadata().is_ok());
            assert!(shim.exists());

            call_tool(
                &sandbox,
                "uninstall_tool",
                json!({ "tool": "protostar", "spec": "1.0.0" }),
            );

            assert!(bin.symlink_metadata().is_err());
            assert!(!shim.exists());
            assert!(load_manifest(&sandbox).installed_versions.is_empty());
        }

        #[test]
        fn removes_version_from_lockfile() {
            let sandbox = create_proto_sandbox("lockfile-uninstall");
            let lockfile_path = sandbox.path().join(".protolock");

            fs::write(
                &lockfile_path,
                fs::read_to_string(&lockfile_path)
                    .unwrap()
                    .replace("{os}", &SystemOS::default().to_string())
                    .replace("{arch}", &SystemArch::default().to_string()),
            )
            .unwrap();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("protostar").arg("5.10.0");
                })
                .success();

            call_tool(
                &sandbox,
                "uninstall_tool",
                json!({ "tool": "protostar", "spec": "5.10.0" }),
            );

            let lockfile = ProtoLock::load(&lockfile_path).unwrap();
            let versions = lockfile
                .tools
                .get("protostar")
                .unwrap()
                .iter()
                .map(|record| record.version.as_ref().unwrap().to_string())
                .collect::<Vec<_>>();

            assert_eq!(versions, ["4.5.15"]);
        }

        #[test]
        fn returns_false_if_not_installed() {
            let sandbox = create_empty_proto_sandbox();

            let result = call_tool(
                &sandbox,
                "uninstall_tool",
                json!({ "tool": "protostar", "spec": "1.0.0" }),
            );

            assert_eq!(result, json!({ "uninstalled": false, "spec": "1.0.0" }));
        }
    }
}
