#![cfg(windows)]

use proto_core::test_utils::create_empty_proto_sandbox;

#[test]
fn exports_posix_paths_for_bash_in_emulated_shells() {
    let sandbox = create_empty_proto_sandbox();

    let assert = sandbox.run_bin(|cmd| {
        cmd.arg("activate").arg("bash").arg("--export");
        cmd.env("MSYSTEM", "MINGW64");
    });

    let output = assert.output();
    let path_line = output
        .lines()
        .find(|line| line.contains("_PROTO_ACTIVATED_PATH"))
        .expect("path export exists");

    assert!(path_line.contains("/.proto/shims"));
    assert!(path_line.contains("/.proto/bin"));
    assert!(!path_line.contains('\\'));
    assert!(path_line.contains('/'));
    assert!(path_line.contains(":"));

    let shims_index = path_line.find("/.proto/shims").unwrap();
    let bin_index = path_line.find("/.proto/bin").unwrap();
    assert!(shims_index < bin_index);
}

#[test]
fn leaves_native_shell_paths_unchanged() {
    let sandbox = create_empty_proto_sandbox();

    let assert = sandbox.run_bin(|cmd| {
        cmd.arg("activate").arg("pwsh").arg("--export");
        cmd.env("MSYSTEM", "MINGW64");
    });

    let output = assert.output();
    let path_line = output
        .lines()
        .find(|line| line.contains("_PROTO_ACTIVATED_PATH"))
        .expect("path export exists");

    assert!(path_line.contains(';'));
    assert!(!path_line.contains("/.proto/shims:/"));
}
