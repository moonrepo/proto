mod utils;

use utils::*;

mod general {
    use super::*;

    #[test]
    fn can_write_to_a_log_file() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("debug")
                    .arg("config")
                    .arg("--log-file")
                    .arg("./proto.log")
                    .arg("--log")
                    .arg("trace");
            })
            .success();

        assert!(sandbox.path().join("proto.log").exists());
    }

    #[test]
    fn can_write_to_a_log_file_with_env_var() {
        let sandbox = create_empty_proto_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("debug")
                    .arg("config")
                    .arg("--log")
                    .arg("trace")
                    .env("PROTO_LOG_FILE", "./proto.log");
            })
            .success();

        assert!(sandbox.path().join("proto.log").exists());
    }
}
