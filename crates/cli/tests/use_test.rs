mod utils;

use utils::*;

#[test]
fn installs_all_tools() {
    let temp = create_empty_sandbox();
    let node_path = temp.path().join("tools/node/19.0.0");
    let npm_path = temp.path().join("tools/npm/9.0.0");
    let deno_path = temp.path().join("tools/deno/1.30.0");

    temp.create_file(
        ".prototools",
        r#"node = "19.0.0"
npm = "9.0.0"
deno = "1.30.0"
"#,
    );

    assert!(!node_path.exists());
    assert!(!npm_path.exists());
    assert!(!deno_path.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("use").assert().success();

    assert!(node_path.exists());
    assert!(npm_path.exists());
    assert!(deno_path.exists());
}

#[test]
fn installs_all_plugins() {
    let temp = create_empty_sandbox_with_tools();
    let moon_path = temp.path().join("tools/moon-test/1.0.0");

    assert!(!moon_path.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("use").assert().success();

    assert!(moon_path.exists());
}

#[test]
fn installs_tool_via_detection() {
    let temp = create_empty_sandbox_with_tools();
    let node_path = temp.path().join("tools/node/19.0.0");

    temp.create_file(".nvmrc", "19.0.0");

    assert!(!node_path.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("use").assert().success();

    assert!(node_path.exists());
}
