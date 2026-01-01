mod utils;

#[cfg(unix)]
mod upgrade {
    use super::utils::*;
    use proto_shim::get_exe_file_name;

    #[test]
    fn upgrades_to_a_version() {
        let sandbox = create_empty_proto_sandbox();
        let main_exe = get_exe_file_name("proto");
        let shim_exe = get_exe_file_name("proto-shim");

        sandbox
            .run_bin(|cmd| {
                cmd.arg("upgrade").arg("0.39.0");
            })
            .success();

        assert!(sandbox.path().join(".proto/bin").join(main_exe).exists());
        assert!(sandbox.path().join(".proto/bin").join(shim_exe).exists());
    }
}
