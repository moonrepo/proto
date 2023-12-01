use shared_child::SharedChild;
use std::process::Command;
use std::sync::Arc;
use std::{env, process};

pub fn main() {
    dbg!("Args", env::args().collect::<Vec<_>>());
    dbg!("Args OS", env::args_os().collect::<Vec<_>>());
    dbg!("Exec with", env::current_exe().unwrap());

    let mut command = Command::new("node");
    command.arg("-e");
    command.arg(
        r#"
console.log('start');

process.on('SIGINT', function() {
	console.log('killed');
	process.exit(1);
});

setTimeout(() => { console.log('stop'); }, 5000);"#,
    );

    let shared_child = SharedChild::spawn(&mut command).unwrap();
    let child = Arc::new(shared_child);
    let child_clone = Arc::clone(&child);

    ctrlc::set_handler(move || {
        println!("Ctrl-C!");
        child_clone.kill().unwrap();
    })
    .unwrap();

    let status = child.wait().unwrap();

    println!("Status = {}", status);

    process::exit(status.code().unwrap_or(0))
}
