use std::env;

pub fn main() {
    dbg!("Args", env::args().collect::<Vec<_>>());
    dbg!("Args OS", env::args_os().collect::<Vec<_>>());
    dbg!("Exec with", env::current_exe().unwrap());
}
