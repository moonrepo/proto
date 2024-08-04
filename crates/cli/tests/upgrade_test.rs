mod utils;

use utils::*;

mod upgrade {
    use super::*;

    #[test]
    fn upgrades_to_a_version() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("upgrade").arg("0.39.0");
            })
            .success();

        assert!(sandbox.path().join(".proto/bin/proto").exists());
        assert!(sandbox.path().join(".proto/bin/proto-shim").exists());
    }

    #[test]
    fn relocates_existing_to_tools_dir() {
        let sandbox = create_empty_proto_sandbox();
        let version = env!("CARGO_PKG_VERSION");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("upgrade").arg("0.39.0");
            })
            .success();

        assert!(sandbox
            .path()
            .join(".proto/tools/proto")
            .join(version)
            .join("proto")
            .exists());
        assert!(sandbox
            .path()
            .join(".proto/tools/proto")
            .join(version)
            .join("proto-shim")
            .exists());
    }
}
