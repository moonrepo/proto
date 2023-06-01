// mod utils;

// use utils::*;

// #[test]
// fn installs_node_global() {
//     let temp = create_empty_sandbox();

//     let mut cmd = create_proto_command(temp.path());
//     cmd.arg("install").arg("npm").arg("latest").assert();

//     let mut cmd = create_proto_command(temp.path());
//     let assert = cmd
//         .arg("install-global")
//         .arg("node")
//         .arg("typescript")
//         .assert();

//     let output = output_to_string(&assert.get_output().stderr.to_vec());

//     dbg!(&output);

//     assert!(temp.path().join("tools/node/globals/bin/tsc").exists());
// }
