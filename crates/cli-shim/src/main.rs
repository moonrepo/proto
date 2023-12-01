use shared_child::SharedChild;
use std::collections::VecDeque;
use std::io::{IsTerminal, Read, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::{env, io, process};

pub fn main() {
    sigpipe::reset();

    // Extract arguments to pass-through
    let mut args = env::args().collect::<VecDeque<_>>();
    let shim_name = args.pop_front().unwrap();

    dbg!("Args", &args);
    dbg!("Shim name", shim_name);
    dbg!("Exec with", env::current_exe().unwrap());

    // Capture any piped input
    let input = {
        let mut stdin = io::stdin();
        let mut buffer = String::new();

        // Only read piped data when stdin is not a TTY,
        // otherwise the process will hang indefinitely waiting for EOF
        if !stdin.is_terminal() {
            stdin.read_to_string(&mut buffer).unwrap();
        }

        buffer
    };
    let has_piped_stdin = !input.is_empty();

    dbg!("Input", &input);

    // The actual command to execute
    let mut command = Command::new("node");
    command.arg("./docs/shim-test.mjs");
    command.args(args);

    if has_piped_stdin {
        command.stdin(Stdio::piped());
    }

    // Spawn a shareable child process
    let shared_child = SharedChild::spawn(&mut command).unwrap();
    let child = Arc::new(shared_child);
    let child_clone = Arc::clone(&child);

    // Handle CTRL+C and kill the child
    ctrlc::set_handler(move || {
        child_clone.kill().unwrap();
    })
    .unwrap();

    // If we have piped data, pass it through
    if has_piped_stdin {
        if let Some(mut stdin) = child.take_stdin() {
            stdin.write_all(input.as_bytes()).unwrap();
            drop(stdin);
        }
    }

    // Wait for the process to finish or be killed
    let status = child.wait().unwrap();
    let code = status.code().unwrap_or(0);

    println!("status code = {}", code);

    process::exit(code);
}
