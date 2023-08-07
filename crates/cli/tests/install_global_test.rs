// mod utils;

// use utils::*;

// mod install_global {
//     use super::*;

//     #[test]
//     fn installs_node_global() {
//         let temp = create_empty_sandbox();

//         let mut cmd = create_proto_command(temp.path());
//         cmd.arg("install").arg("npm").arg("latest").assert();

//         let mut cmd = create_proto_command(temp.path());
//         let assert = cmd
//             .arg("install-global")
//             .arg("npm")
//             .arg("typescript")
//             .assert();

//         let output = output_to_string(&assert.get_output().stderr.to_vec());

//         println!("{}", &output);

//         assert!(temp.path().join("tools/node/globals/bin/tsc").exists());
//     }
// }
