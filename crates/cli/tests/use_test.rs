mod utils;

use predicates::prelude::*;
use std::fs;
use utils::*;

#[test]
fn errors_if_no_config() {
    let temp = create_temp_dir();

    let mut cmd = create_proto_command(temp.path());
    let assert = cmd.arg("use").assert();

    assert.stderr(predicate::str::contains(
        "Could not locate a .prototools configuration file.",
    ));
}

#[test]
fn installs_all_tools() {
    let temp = create_temp_dir();
    let node_path = temp.join("tools/node/19.0.0");
    let npm_path = temp.join("tools/npm/9.0.0");
    let deno_path = temp.join("tools/deno/1.30.0");

    fs::write(
        temp.path().join(".prototools"),
        r#"node = "19.0.0"
npm = "9.0.0"
deno = "1.30.0"
"#,
    )
    .unwrap();

    assert!(!node_path.exists());
    assert!(!npm_path.exists());
    assert!(!deno_path.exists());

    let mut cmd = create_proto_command(temp.path());
    cmd.arg("use").assert();

    assert!(node_path.exists());
    assert!(npm_path.exists());
    assert!(deno_path.exists());
}
