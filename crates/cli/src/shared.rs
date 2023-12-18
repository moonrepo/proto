// This code is shared between the shim and main binaries!

use shared_child::SharedChild;
use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};
use std::process::Command;
use std::sync::Arc;
use std::{io, thread};

pub fn spawn_command_with_signals(mut command: Command) -> io::Result<Arc<SharedChild>> {
    let shared_child = SharedChild::spawn(&mut command)?;
    let child = Arc::new(shared_child);
    let child_clone = Arc::clone(&child);

    thread::spawn(move || {
        let mut signals = Signals::new(TERM_SIGNALS).unwrap();

        for signal in signals.forever() {
            #[cfg(not(windows))]
            {
                use shared_child::unix::SharedChildExt;

                let a = child_clone.send_signal(signal);

                dbg!(&a);

                if a.is_err() {
                    let _ = child_clone.kill();
                }
            }

            #[cfg(windows)]
            {
                let _ = child_clone.kill();
            }

            break;
        }
    });

    Ok(child)
}
