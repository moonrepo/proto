mod utils;

use proto_core::{ToolContext, UnresolvedVersionSpec};
use std::fs;
use utils::*;

mod pin_local {
    use super::*;

    #[test]
    fn writes_local_version_file() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        assert!(!version_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("protostar").arg("1.0.0");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "protostar = \"1.0.0\"\n"
        )
    }

    #[test]
    fn appends_multiple_tools() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("protostar").arg("1.0.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("moonstone").arg("2.0.0");
            })
            .success();

        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            r#"protostar = "1.0.0"
moonstone = "2.0.0"
"#
        )
    }

    #[test]
    fn will_overwrite_by_name() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        sandbox.create_file(
            ".prototools",
            r#"protostar = "1.0.0"
moonstone = "2.0.0"
"#,
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("protostar").arg("2");
            })
            .success();

        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            r#"protostar = "~2"
moonstone = "2.0.0"
"#
        );
    }

    #[test]
    fn can_set_aliases() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        assert!(!version_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("protostar").arg("bundled");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "protostar = \"bundled\"\n"
        )
    }

    #[test]
    fn can_set_partial_version() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        assert!(!version_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("protostar").arg("1.2");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "protostar = \"~1.2\"\n"
        )
    }

    #[test]
    fn can_resolve_partial_version() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        assert!(!version_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("protostar").arg("5").arg("--resolve");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "protostar = \"5.10.15\"\n"
        )
    }

    #[test]
    fn can_set_proto() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        assert!(!version_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("proto").arg("0.45.0");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "proto = \"0.45.0\"\n"
        )
    }

    // Windows doesn't support asdf
    #[cfg(unix)]
    mod backend {
        use super::*;

        #[test]
        fn can_set() {
            let sandbox = create_empty_proto_sandbox();
            let version_file = sandbox.path().join(".prototools");

            assert!(!version_file.exists());

            sandbox
                .run_bin(|cmd| {
                    cmd.arg("pin").arg("asdf:act").arg("0.2.70");
                })
                .success();

            assert!(version_file.exists());
            assert_eq!(
                fs::read_to_string(version_file).unwrap(),
                "\"asdf:act\" = \"0.2.70\"\n"
            )
        }
    }
}

mod pin_global {
    use super::*;

    #[test]
    fn updates_manifest_file() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join(".proto/.prototools");

        assert!(!config_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin")
                    .arg("--to")
                    .arg("global")
                    .arg("protostar")
                    .arg("1.0.0");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config
                .versions
                .get(&ToolContext::parse("protostar").unwrap())
                .unwrap(),
            &UnresolvedVersionSpec::parse("1.0.0").unwrap()
        );
    }

    #[test]
    fn can_set_alias_as_default() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join(".proto/.prototools");

        assert!(!config_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin")
                    .arg("--to")
                    .arg("global")
                    .arg("protostar")
                    .arg("bundled");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config
                .versions
                .get(&ToolContext::parse("protostar").unwrap())
                .unwrap(),
            &UnresolvedVersionSpec::Alias("bundled".into())
        );
    }

    #[test]
    fn can_set_partial_version_as_default() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join(".proto/.prototools");

        assert!(!config_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin")
                    .arg("--to")
                    .arg("global")
                    .arg("protostar")
                    .arg("1.2");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config
                .versions
                .get(&ToolContext::parse("protostar").unwrap())
                .unwrap(),
            &UnresolvedVersionSpec::parse("1.2").unwrap()
        );
    }

    #[test]
    fn can_resolve_partial_version() {
        let sandbox = create_empty_proto_sandbox();
        let config_file = sandbox.path().join(".proto/.prototools");

        assert!(!config_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin")
                    .arg("--to")
                    .arg("global")
                    .arg("protostar")
                    .arg("5")
                    .arg("--resolve");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config
                .versions
                .get(&ToolContext::parse("protostar").unwrap())
                .unwrap(),
            &UnresolvedVersionSpec::parse("5.10.15").unwrap()
        );
    }

    #[test]
    fn doesnt_create_bin_symlink() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin")
                    .arg("--to")
                    .arg("global")
                    .arg("protostar")
                    .arg("1");
            })
            .success();

        let link = get_bin_path(sandbox.path(), "protostar");

        assert!(!link.exists());
    }

    #[test]
    fn can_set_proto() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".proto/.prototools");

        assert!(!version_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin")
                    .arg("proto")
                    .arg("0.45.0")
                    .arg("--to")
                    .arg("global");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "proto = \"0.45.0\"\n"
        )
    }
}

mod pin_user {
    use super::*;

    #[test]
    fn writes_user_version_file() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".home/.prototools");

        assert!(!version_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin")
                    .arg("protostar")
                    .arg("1.0.0")
                    .arg("--to")
                    .arg("home");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "protostar = \"1.0.0\"\n"
        )
    }

    #[test]
    fn can_set_proto() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".home/.prototools");

        assert!(!version_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin")
                    .arg("proto")
                    .arg("0.45.0")
                    .arg("--to")
                    .arg("home");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "proto = \"0.45.0\"\n"
        )
    }
}
