mod utils;

use std::fs;
use std::path::Path;
use utils::*;

fn install_node(sandbox: &Path) {
    let mut cmd = create_proto_command(sandbox);
    cmd.arg("install")
        .arg("node")
        .arg("--pin")
        .arg("--")
        .arg("--no-bundled-npm")
        .assert()
        .success();
}

mod regen_shim {
    use super::*;

    #[test]
    fn replaces_existing_shims() {
        let sandbox = create_empty_sandbox();

        install_node(sandbox.path());

        let old_timestamp = fs::metadata(get_shim_path(sandbox.path(), "node"))
            .unwrap()
            .created()
            .unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("regen").assert().success();

        let new_timestamp = fs::metadata(get_shim_path(sandbox.path(), "node"))
            .unwrap()
            .created()
            .unwrap();

        assert_ne!(old_timestamp, new_timestamp);
    }

    #[test]
    fn doesnt_create_shims_for_noninstalled_tools() {
        let sandbox = create_empty_sandbox();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("regen").assert().success();

        assert!(!get_shim_path(sandbox.path(), "go").exists());
        assert!(!get_shim_path(sandbox.path(), "node").exists());
    }

    #[test]
    fn deletes_unknown_shims() {
        let sandbox = create_empty_sandbox();
        let unknown_path = get_shim_path(sandbox.path(), "unknown");

        fs::create_dir_all(unknown_path.parent().unwrap()).unwrap();
        fs::write(&unknown_path, "shim").unwrap();

        let mut cmd = create_proto_command(sandbox.path());
        cmd.arg("regen").assert().success();

        assert!(!unknown_path.exists());
    }
}
