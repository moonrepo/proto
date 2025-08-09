mod utils;

use starbase_sandbox::{Sandbox, assert_snapshot, predicates::prelude::*};
use std::{env, fs};
use utils::*;

fn install_node(sandbox: &Sandbox) {
    sandbox
        .run_bin(|cmd| {
            cmd.arg("install").arg("node").arg("19.0.0");
        })
        .success();
}

mod run {
    use super::*;

    #[test]
    fn doesnt_error_for_tools_on_path_but_not_configured_in_proto() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("git");
            })
            // `git` with no args is exit 1
            .failure();

        assert.stdout(predicate::str::contains("usage: git"));
    }

    #[test]
    fn errors_if_not_installed() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("node").arg("19.0.0");
            })
            .failure();

        assert.stderr(predicate::str::contains(
            "This project requires Node.js 19.0.0",
        ));
    }

    #[test]
    fn errors_if_no_version_detected() {
        let sandbox = create_empty_proto_sandbox();

        // Note that moon must not be installed in the system without proto for this test to pass.
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("moon");
            })
            .failure();

        assert.stderr(predicate::str::contains(
            "Failed to detect an applicable version",
        ));
    }

    #[test]
    fn runs_tool_from_path_if_proto_fails() {
        let sandbox = create_empty_proto_sandbox();

        // Note that node must be installed in the system without proto for this test to pass.
        // In github CI task runners this is usually the case.
        sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("node").arg("--version");
            })
            .success();
    }

    #[test]
    fn runs_a_tool() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("19.0.0");
            })
            .success();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("--version");
            })
            .success();

        assert.stdout(predicate::str::contains("19.0.0"));
    }

    #[test]
    fn runs_a_tool_using_version_detection() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("19.0.0");
            })
            .success();

        // Arg
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("--version");
            })
            .success();

        assert.stdout(predicate::str::contains("19.0.0"));

        // Env var
        let assert = sandbox
            .run_bin(|cmd| {
                cmd.env("PROTO_NODE_VERSION", "19.0.0")
                    .arg("run")
                    .arg("node")
                    .arg("--")
                    .arg("--version");
            })
            .success();

        assert.stdout(predicate::str::contains("19.0.0"));

        // Local version
        sandbox.create_file(".prototools", "node = \"19.0.0\"");

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("node").arg("--").arg("--version");
            })
            .success();

        assert.stdout(predicate::str::contains("19.0.0"));

        fs::remove_file(sandbox.path().join(".prototools")).unwrap();

        // Global version
        sandbox.create_file(".proto/.prototools", "node = \"19.0.0\"");

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("node").arg("--").arg("--version");
            })
            .success();

        assert.stdout(predicate::str::contains("19.0.0"));
    }

    #[test]
    fn updates_last_used_at() {
        let sandbox = create_empty_proto_sandbox();
        let last_used_file = sandbox.path().join(".proto/tools/node/19.0.0/.last-used");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("node").arg("19.0.0");
            })
            .success();

        assert!(!last_used_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("--version");
            })
            .success();

        let value = fs::read_to_string(&last_used_file).unwrap();

        assert!(last_used_file.exists());
        assert_ne!(value, "");

        // Run again and make sure timestamps update
        sandbox
            .run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("--version");
            })
            .success();

        let new_value = fs::read_to_string(&last_used_file).unwrap();

        assert!(last_used_file.exists());
        assert_ne!(value, new_value);
    }

    #[test]
    fn auto_installs_if_missing() {
        let sandbox = create_empty_proto_sandbox();

        sandbox.create_file(".prototools", "[settings]\nauto-install = true");

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("-e")
                    .arg("'//'");
            })
            .success();

        assert.stdout(predicate::str::contains("installed"));
    }

    #[test]
    fn auto_installs_if_missing_with_env_var() {
        let sandbox = create_empty_proto_sandbox();

        unsafe { env::set_var("PROTO_AUTO_INSTALL", "true") };

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("-e")
                    .arg("'//'");
            })
            .success();

        assert.stdout(predicate::str::contains("installed"));

        unsafe { env::remove_var("PROTO_AUTO_INSTALL") };
    }

    #[test]
    fn doesnt_auto_install_if_false() {
        let sandbox = create_empty_proto_sandbox();

        sandbox.create_file(".prototools", "[settings]\nauto-install = false");

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("-e")
                    .arg("'//'");
            })
            .failure();

        assert.stderr(predicate::str::contains(
            "This project requires Node.js 19.0.0",
        ));
    }

    #[test]
    fn doesnt_auto_install_subsequently() {
        let sandbox = create_empty_proto_sandbox();

        sandbox.create_file(".prototools", "[settings]\nauto-install = true");

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("-e")
                    .arg("'//'");
            })
            .success();

        assert.stdout(predicate::str::contains("Node.js 19.0.0 installed"));

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("-e")
                    .arg("'//'");
            })
            .success();

        assert.stdout(predicate::str::contains("Node.js 19.0.0 installed").not());
    }

    #[test]
    fn errors_if_plugin_not_configured() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("plugin-name").arg("1.0.0");
            })
            .failure();

        assert.stderr(predicate::str::contains(
            "plugin-name is not a built-in plugin",
        ));
    }

    mod env_vars {
        use super::*;

        #[test]
        fn inherits_from_config() {
            let sandbox = create_proto_sandbox("env-vars");

            sandbox.create_file(
                ".prototools",
                r#"
[tools.node.env]
FROM_CONFIG = "abc123"
FROM_CONFIG_BOOL = true
"#,
            );

            install_node(&sandbox);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("test.js");
            });

            assert_snapshot!(assert.output_standardized());
        }

        #[test]
        fn inherits_from_parent() {
            let sandbox = create_proto_sandbox("env-vars");

            install_node(&sandbox);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("test.js")
                    .env("FROM_PARENT", "abc123");
            });

            assert_snapshot!(assert.output_standardized());
        }

        #[test]
        fn can_disable_inherits_from_parent_with_config() {
            let sandbox = create_proto_sandbox("env-vars");

            sandbox.create_file(
                ".prototools",
                r#"
[tools.node.env]
FROM_PARENT_REMOVED = false
"#,
            );

            install_node(&sandbox);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("test.js")
                    .env("FROM_PARENT", "abc123")
                    .env("FROM_PARENT_REMOVED", "abc123");
            });

            assert_snapshot!(assert.output_standardized());
        }

        #[test]
        fn parent_overrides_config() {
            let sandbox = create_proto_sandbox("env-vars");

            sandbox.create_file(
                ".prototools",
                r#"
[tools.node.env]
FROM_CONFIG = "abc123"
"#,
            );

            install_node(&sandbox);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("test.js")
                    .env("FROM_CONFIG", "xyz789")
                    .env("FROM_PARENT", "xyz789");
            });

            assert_snapshot!(assert.output_standardized());
        }

        #[test]
        fn supports_interpolation() {
            let sandbox = create_proto_sandbox("env-vars");

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

            install_node(&sandbox);

            let assert = sandbox.run_bin(|cmd| {
                cmd.arg("run")
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--")
                    .arg("interpolation.js")
                    .env("SECOND", "789")
                    .env("PARENT", "xyz");
            });

            assert_snapshot!(assert.output_standardized());
        }
    }

    mod proto {
        use super::*;

        #[test]
        fn runs_the_global_exe_if_nothing_installed() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("run").arg("proto").arg("--").arg("--version");
                })
                .success();

            assert.stdout(predicate::str::contains("0.45.0").not());
        }

        #[test]
        fn runs_the_installed_exe() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("proto").arg("0.45.0");
                })
                .success();

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("run")
                        .arg("proto")
                        .arg("0.45.0")
                        .arg("--")
                        .arg("--version");
                })
                .success();

            assert.stdout(predicate::str::contains("0.45.0"));
        }

        #[test]
        fn runs_using_version_detection() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("proto").arg("0.45.0");
                })
                .success();

            // Env var
            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.env("PROTO_PROTO_VERSION", "0.45.0")
                        .arg("run")
                        .arg("proto")
                        .arg("--")
                        .arg("--version");
                })
                .success();

            assert.stdout(predicate::str::contains("0.45.0"));

            // Local version
            sandbox.create_file(".prototools", "proto = \"0.45.0\"");

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("run").arg("proto").arg("--").arg("--version");
                })
                .success();

            assert.stdout(predicate::str::contains("0.45.0"));

            fs::remove_file(sandbox.path().join(".prototools")).unwrap();

            // Global version
            sandbox.create_file(".proto/.prototools", "proto = \"0.45.0\"");

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("run").arg("proto").arg("--").arg("--version");
                })
                .success();

            assert.stdout(predicate::str::contains("0.45.0"));
        }
    }

    #[cfg(not(windows))]
    mod backend {
        use super::*;

        #[test]
        fn errors_if_not_installed() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("run").arg("asdf:zig").arg("0.13");
                })
                .failure();

            assert.stderr(predicate::str::contains(
                "This project requires asdf:zig ~0.13",
            ));
        }

        #[test]
        fn errors_if_no_version_detected() {
            let sandbox = create_empty_proto_sandbox();

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("run").arg("asdf:zig");
                })
                .failure();

            assert.stderr(predicate::str::contains(
                "Failed to detect an applicable version",
            ));
        }

        #[test]
        fn runs_a_tool() {
            let sandbox = create_empty_proto_sandbox();

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("asdf:zig").arg("0.13.0");
                })
                .success();

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("run")
                        .arg("asdf:zig")
                        .arg("0.13.0")
                        .arg("--")
                        .arg("version");
                })
                .success();

            assert.stdout(predicate::str::contains("0.13.0"));
        }

        #[test]
        fn runs_a_tool_using_version_detection() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".prototools", "\"asdf:zig\" = \"0.13.0\"");

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("install").arg("asdf:zig").arg("0.13.0");
                })
                .success();

            let assert = sandbox
                .run_bin(|cmd| {
                    cmd.arg("run").arg("asdf:zig").arg("--").arg("version");
                })
                .success();

            assert.stdout(predicate::str::contains("0.13.0"));
        }
    }
}
