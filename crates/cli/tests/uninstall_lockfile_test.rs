mod utils;

use proto_core::ProtoLock;
use utils::*;

mod uninstall_lockfile {
    use super::*;

    #[test]
    fn removes_matching_version_from_file() {
        let sandbox = create_proto_sandbox("lockfile-uninstall");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("5.10.0");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("protostar")
                    .arg("5.10.0")
                    .arg("--yes");
            })
            .success();

        let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
        let records = lockfile.tools.get("protostar").unwrap();

        assert_eq!(records.len(), 1);
    }

    #[test]
    fn doesnt_remove_spec_from_file_even_if_versions_match() {
        let sandbox = create_proto_sandbox("lockfile-uninstall");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("4.5.15");
            })
            .success();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("protostar")
                    .arg("4.5.15")
                    .arg("--yes");
            })
            .success();

        let lockfile = ProtoLock::load(sandbox.path().join(".protolock")).unwrap();
        let records = lockfile.tools.get("protostar").unwrap();

        assert_eq!(records.len(), 2);
    }

    #[test]
    fn deletes_file_if_no_contents() {
        let sandbox = create_proto_sandbox("lockfile");
        let lockfile_path = sandbox.path().join(".protolock");

        assert!(!lockfile_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("install").arg("protostar").arg("5.10.0");
            })
            .success();

        assert!(lockfile_path.exists());

        sandbox
            .run_bin(|cmd| {
                cmd.arg("uninstall")
                    .arg("protostar")
                    .arg("5.10.0")
                    .arg("--yes");
            })
            .success();

        assert!(!lockfile_path.exists());
    }
}
