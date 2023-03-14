#![allow(dead_code)]

use std::path::Path;

pub fn output_to_string(data: &[u8]) -> String {
    String::from_utf8(data.to_vec()).unwrap_or_default()
}

pub fn create_temp_dir() -> assert_fs::TempDir {
    assert_fs::TempDir::new().unwrap()
}

pub fn create_proto_command<T: AsRef<Path>>(path: T) -> assert_cmd::Command {
    let path = path.as_ref();

    let mut cmd = assert_cmd::Command::cargo_bin("proto").unwrap();
    // cmd.timeout(std::time::Duration::from_secs(120));
    cmd.current_dir(path);
    cmd.env("RUST_BACKTRACE", "1");
    cmd.env("PROTO_ROOT", path.as_os_str());
    cmd
}

pub fn debug_assert(assert: &assert_cmd::assert::Assert) {
    let output = assert.get_output();

    println!("STDOUT:\n{}\n", output_to_string(&output.stdout));
    println!("STDERR:\n{}\n", output_to_string(&output.stderr));
    println!("STATUS:\n{:#?}", output.status);
}
