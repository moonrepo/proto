//! The default soft limit for open file descriptors is low on some platforms
//! (256 on macOS, 1024 on most Linux distros), and proto can exceed it when
//! many tools are loaded at once, as every WASM plugin holds a descriptor for
//! each of its preopened directories. The hard limit is usually much higher,
//! so raise the soft limit at startup, as Go and uv do, and restore the
//! original before executing a tool, so that tools inherit the limit of the
//! environment that proto was started in.
//!
//! @see https://github.com/moonrepo/proto/issues/1178

use std::io;
use std::process::Command;

#[cfg(unix)]
mod unix {
    use libc::rlim_t;
    use std::io;
    use std::sync::OnceLock;
    use tracing::{debug, trace};

    // Some programs break when the limit exceeds `i32::MAX`, or take
    // a long time when they loop over every possible descriptor
    // @see https://bugs.openjdk.org/browse/JDK-8324577
    const MAX_LIMIT: rlim_t = 0x10_0000;

    // Older macOS versions reject a soft limit above `OPEN_MAX`
    #[cfg(target_os = "macos")]
    const OPEN_MAX: rlim_t = 10240;

    static ORIGINAL_LIMIT: OnceLock<rlim_t> = OnceLock::new();

    /// Return the soft and hard limits.
    fn get_limit() -> io::Result<(rlim_t, rlim_t)> {
        let mut limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };

        // SAFETY: `limit` is a valid and writable `rlimit` for the duration of the call
        if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) } != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok((limit.rlim_cur, limit.rlim_max))
    }

    fn set_limit(soft: rlim_t, hard: rlim_t) -> io::Result<()> {
        let limit = libc::rlimit {
            rlim_cur: soft,
            rlim_max: hard,
        };

        // SAFETY: `limit` is a valid `rlimit` for the duration of the call
        if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) } != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    fn raise() -> io::Result<Option<(rlim_t, rlim_t)>> {
        let (soft, hard) = get_limit()?;

        // `RLIM_INFINITY` is the max value, so is capped as well
        let target = hard.min(MAX_LIMIT);

        if soft >= target {
            return Ok(None);
        }

        let result = set_limit(target, hard);

        #[cfg(target_os = "macos")]
        let (result, target) = match result {
            Err(error)
                if error.raw_os_error() == Some(libc::EINVAL)
                    && soft < OPEN_MAX
                    && target > OPEN_MAX =>
            {
                (set_limit(OPEN_MAX, hard), OPEN_MAX)
            }
            other => (other, target),
        };

        result?;

        let _ = ORIGINAL_LIMIT.set(soft);

        Ok(Some((soft, target)))
    }

    pub fn raise_open_files_limit() {
        // The default limit may still be enough, so never fail
        match raise() {
            Ok(Some((from, to))) => {
                debug!(from, to, "Raised the soft limit for open file descriptors");
            }
            Ok(None) => {
                trace!("Soft limit for open file descriptors is already high enough");
            }
            Err(error) => {
                debug!("Failed to raise the soft limit for open file descriptors: {error}");
            }
        };
    }

    pub fn restore_open_files_limit() {
        let Some(original) = ORIGINAL_LIMIT.get() else {
            return;
        };

        if let Ok((_, hard)) = get_limit() {
            let _ = set_limit(*original, hard);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // The limit is process-wide, so must be tested sequentially in a single test
        #[test]
        fn raises_and_restores_soft_limit() {
            let (initial_soft, hard) = get_limit().unwrap();

            if hard < 256 {
                return;
            }

            set_limit(256, hard).unwrap();

            raise_open_files_limit();

            let (raised, _) = get_limit().unwrap();

            assert!(raised > 256);
            assert!(raised <= hard.min(MAX_LIMIT));

            restore_open_files_limit();

            assert_eq!(get_limit().unwrap(), (256, hard));

            // Already sufficient, so doesn't change
            set_limit(hard.min(MAX_LIMIT), hard).unwrap();

            assert!(matches!(raise(), Ok(None)));

            set_limit(initial_soft, hard).unwrap();
        }

        #[derive(Debug, thiserror::Error)]
        #[error("Failed to create lock")]
        struct WrappedError(#[source] io::Error);

        #[test]
        fn detects_open_files_limit_errors() {
            use crate::utils::open_files::is_open_files_limit_error;

            let emfile = io::Error::from_raw_os_error(libc::EMFILE);

            assert!(is_open_files_limit_error(&emfile));
            assert!(is_open_files_limit_error(&WrappedError(emfile)));
            // Plugin errors are strings
            assert!(is_open_files_limit_error(&io::Error::other(
                "Too many open files (os error 24)"
            )));
            assert!(!is_open_files_limit_error(&WrappedError(
                io::Error::from_raw_os_error(libc::ENOENT)
            )));
        }
    }
}

#[cfg(unix)]
pub use unix::{raise_open_files_limit, restore_open_files_limit};

#[cfg(not(unix))]
pub fn raise_open_files_limit() {}

#[cfg(not(unix))]
pub fn restore_open_files_limit() {}

/// Return true if the error, or one of its causes, is from reaching the limit
/// for open file descriptors (`EMFILE`). Errors from plugins are converted to
/// strings, so match against the message instead of downcasting.
pub fn is_open_files_limit_error(error: &dyn std::error::Error) -> bool {
    cfg!(unix)
        && std::iter::successors(Some(error), |error| error.source())
            .any(|error| error.to_string().contains("(os error 24)"))
}

/// Restore the original open files limit, then execute the command
/// and replace the current process (on Unix).
pub fn exec_command_and_replace(command: Command) -> io::Result<()> {
    restore_open_files_limit();

    proto_shim::exec_command_and_replace(command)
}
