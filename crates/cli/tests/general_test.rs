mod utils;

use utils::*;

mod systems {
    use super::*;

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
