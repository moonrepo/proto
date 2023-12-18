// This code is shared between the shim and main binaries!

use shared_child::SharedChild;
use signal_hook::consts::*;
use std::process::Command;
use std::sync::Arc;
use std::{io, thread};

// https://blog.logrocket.com/guide-signal-handling-rust/
pub fn spawn_command_with_signals(mut command: Command) -> io::Result<Arc<SharedChild>> {
    let shared_child = SharedChild::spawn(&mut command)?;
    let child = Arc::new(shared_child);
    let child_clone = Arc::clone(&child);

    #[cfg(not(windows))]
    thread::spawn(move || {
        use shared_child::unix::SharedChildExt;
        use signal_hook::iterator::Signals;

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

    #[cfg(windows)]
    thread::spawn(move || {
        use signal_hook::low_level::register;

        for signal in TERM_SIGNALS {
            let child_clone = Arc::clone(&child_clone);

            unsafe {
                register(*signal, move || {
                    let _ = child_clone.kill();
                })
                .unwrap()
            };
        }
    });

    Ok(child)
}
