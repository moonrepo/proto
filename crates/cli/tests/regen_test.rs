mod utils;

use starbase_sandbox::Sandbox;
use std::fs;
use utils::*;

fn install_node(sandbox: &Sandbox) {
    sandbox
        .run_bin(|cmd| {
            cmd.arg("install")
                .arg("node")
                .arg("20.0.0")
                .arg("--pin")
                .arg("--")
                .arg("--no-bundled-npm");
        })
        .success();
}

mod regen_shim {
    use super::*;

    #[test]
    fn replaces_existing_shims() {
        let sandbox = create_empty_proto_sandbox();

        install_node(&sandbox);

        let old_timestamp = fs::metadata(get_shim_path(sandbox.path(), "node"))
            .unwrap()
            .created()
            .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("regen");
            })
            .success();

        let new_timestamp = fs::metadata(get_shim_path(sandbox.path(), "node"))
            .unwrap()
            .created()
            .unwrap();

        assert_ne!(old_timestamp, new_timestamp);
    }

    #[test]
    fn doesnt_create_shims_for_noninstalled_tools() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("regen");
            })
            .success();

        assert!(!get_shim_path(sandbox.path(), "go").exists());
        assert!(!get_shim_path(sandbox.path(), "node").exists());
    }

    #[test]
    fn deletes_unknown_shims() {
        let sandbox = create_empty_proto_sandbox();
        let unknown_path = get_shim_path(sandbox.path(), "unknown");

        fs::create_dir_all(unknown_path.parent().unwrap()).unwrap();
        fs::write(&unknown_path, "shim").unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("regen");
            })
            .success();

        assert!(!unknown_path.exists());
    }
}

mod regen_bin {
    use super::*;

    #[test]
    fn doesnt_replace_bins_by_default() {
        let sandbox = create_empty_proto_sandbox();

        install_node(&sandbox);

        let old_timestamp = fs::metadata(get_bin_path(sandbox.path(), "node"))
            .unwrap()
            .created()
            .unwrap();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("regen");
            })
            .success();

        let new_timestamp = fs::metadata(get_bin_path(sandbox.path(), "node"))
            .unwrap()
            .created()
            .unwrap();

        assert_eq!(old_timestamp, new_timestamp);
    }

    #[test]
    fn deletes_unknown_bins() {
        let sandbox = create_empty_proto_sandbox();
        let base_path = sandbox.path().join("base-bin");

        fs::write(&base_path, "bin").unwrap();

        let unknown_path = get_bin_path(sandbox.path(), "unknown");

        link_bin(&base_path, &unknown_path);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("regen").arg("--bin");
            })
            .success();

        assert!(!unknown_path.exists());
    }

    #[test]
    fn doesnt_delete_proto_bins() {
        let sandbox = create_empty_proto_sandbox();
        let base_path = sandbox.path().join("base-bin");

        fs::write(&base_path, "bin").unwrap();

        let proto_path = get_bin_path(sandbox.path(), "proto");
        let proto_shim_path = get_bin_path(sandbox.path(), "proto-shim");

        link_bin(&base_path, &proto_path);
        link_bin(&base_path, &proto_shim_path);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("regen").arg("--bin");
            })
            .success();

        assert!(proto_path.exists());
        assert!(proto_shim_path.exists());
    }

    #[test]
    fn links_tool_with_bucketed_versions() {
        let sandbox = create_empty_proto_sandbox();

        install_node(&sandbox);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("regen").arg("--bin");
            })
            .success();

        assert!(get_bin_path(sandbox.path(), "node").exists());
        assert!(get_bin_path(sandbox.path(), "node-20").exists());
        assert!(get_bin_path(sandbox.path(), "node-20.0").exists());
    }
}
