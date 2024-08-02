mod utils;

use utils::*;

mod systems {
    use super::*;

    #[test]
    fn copies_current_bin_to_store() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("bin").arg("proto");
            })
            .success();

        // I use a shared target folder for all projects in .cargo,
        // but there's a condition in proto's code to ignore the local
        // debug builds when in cargo. So need to disable this locally.
        if std::env::var("CI").is_ok() {
            assert!(sandbox
                .path()
                .join(".proto/tools/proto")
                .join(env!("CARGO_PKG_VERSION"))
                .exists());
        }
    }

    #[test]
    fn downloads_versioned_bin_to_store() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", r#"proto = "0.30.0""#);

        sandbox
            .run_bin(|cmd| {
                cmd.arg("bin").arg("proto");
            })
            .success();

        assert!(sandbox.path().join(".proto/tools/proto/0.30.0").exists());
    }
}
