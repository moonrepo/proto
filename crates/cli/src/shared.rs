// This code is shared between the shim and main binaries!

use std::io;
use std::process::{Command, ExitStatus};

// On Unix, use native signals.
#[cfg(not(windows))]
pub fn spawn_command_with_signals(
    mut command: Command,
    on_spawn: impl FnOnce(u32),
) -> io::Result<ExitStatus> {
    use shared_child::SharedChild;
    use signal_hook::consts::*;
    use std::sync::Arc;
    use std::thread;

    let shared_child = SharedChild::spawn(&mut command)?;
    let child = Arc::new(shared_child);
    let child_clone = Arc::clone(&child);

    thread::spawn(move || {
        use shared_child::unix::SharedChildExt;
        use signal_hook::iterator::Signals;

        // https://blog.logrocket.com/guide-signal-handling-rust/
        let mut signals =
            Signals::new([SIGTERM, SIGQUIT, SIGINT, SIGHUP, SIGABRT, SIGUSR1, SIGUSR2]).unwrap();

        for signal in signals.forever() {
            let mut sent = child_clone.send_signal(signal).is_ok();

            match signal {
                SIGTERM | SIGQUIT | SIGINT | SIGHUP | SIGABRT => {
                    if !sent {
                        sent = child_clone.kill().is_ok();
                    }

                    if sent {
                        break;
                    }
                }
                _ => {}
            }
        }
    });

    on_spawn(child.id());
    child.wait()
}

// On Windows, use job objects.
#[cfg(windows)]
pub fn spawn_command_with_signals(
    mut command: Command,
    on_spawn: impl FnOnce(u32),
) -> io::Result<ExitStatus> {
    use command_group::CommandGroup;

    let mut group = command.group();
    group.kill_on_drop(true);

    let mut child = group.spawn()?;
    on_spawn(child.id());
    child.wait()
}
