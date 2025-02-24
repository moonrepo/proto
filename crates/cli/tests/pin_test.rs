mod utils;

use proto_core::UnresolvedVersionSpec;
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
                cmd.arg("pin").arg("node").arg("19.0.0");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "node = \"19.0.0\"\n"
        )
    }

    #[test]
    fn appends_multiple_tools() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("node").arg("19.0.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("npm").arg("9.0.0");
            })
            .success();

        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            r#"node = "19.0.0"
npm = "9.0.0"
"#
        )
    }

    #[test]
    fn will_overwrite_by_name() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        sandbox.create_file(
            ".prototools",
            r#"node = "16.0.0"
npm = "9.0.0"
"#,
        );

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("node").arg("19");
            })
            .success();

        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            r#"node = "~19"
npm = "9.0.0"
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
                cmd.arg("pin").arg("npm").arg("bundled");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "npm = \"bundled\"\n"
        )
    }

    #[test]
    fn can_set_partial_version() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        assert!(!version_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("npm").arg("1.2");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "npm = \"~1.2\"\n"
        )
    }

    #[test]
    fn can_resolve_partial_version() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        assert!(!version_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("npm").arg("6").arg("--resolve");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "npm = \"6.14.18\"\n"
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

    #[test]
    fn can_set_asdf_backend() {
        let sandbox = create_empty_proto_sandbox();
        let version_file = sandbox.path().join(".prototools");

        assert!(!version_file.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("pin").arg("act").arg("asdf:0.2.70");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "act = \"asdf:0.2.70\"\n"
        )
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
                    .arg("node")
                    .arg("19.0.0");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config.versions.get("node").unwrap(),
            &UnresolvedVersionSpec::parse("19.0.0").unwrap()
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
                    .arg("npm")
                    .arg("bundled");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config.versions.get("npm").unwrap(),
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
                    .arg("npm")
                    .arg("1.2");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config.versions.get("npm").unwrap(),
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
                    .arg("npm")
                    .arg("6")
                    .arg("--resolve");
            })
            .success();

        assert!(config_file.exists());

        let config = load_config(sandbox.path().join(".proto"));

        assert_eq!(
            config.versions.get("npm").unwrap(),
            &UnresolvedVersionSpec::parse("6.14.18").unwrap()
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
                    .arg("node")
                    .arg("20");
            })
            .success();

        let link = get_bin_path(sandbox.path(), "node");

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
                    .arg("node")
                    .arg("19.0.0")
                    .arg("--to")
                    .arg("home");
            })
            .success();

        assert!(version_file.exists());
        assert_eq!(
            fs::read_to_string(version_file).unwrap(),
            "node = \"19.0.0\"\n"
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
