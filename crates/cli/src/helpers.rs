use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Duration;

pub fn enable_logging() {
    static ENABLED: AtomicBool = AtomicBool::new(false);

    if !ENABLED.load(Relaxed) {
        if let Ok(level) = env::var("PROTO_LOG") {
            if !level.starts_with("proto=") && level != "off" {
                env::set_var("PROTO_LOG", format!("proto={level}"));
            }
        } else {
            env::set_var("PROTO_LOG", "proto=info");
        }

        env_logger::Builder::from_env("PROTO_LOG")
            .format_timestamp(None)
            .init();

        ENABLED.store(true, Relaxed);
    }
}

pub fn create_progress_bar<S: AsRef<str>>(start: S) -> ProgressBar {
    let pb = if env::var("PROTO_NO_PROGRESS").is_ok() {
        ProgressBar::hidden()
    } else {
        ProgressBar::new_spinner()
    };

    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message(start.as_ref().to_owned());
    pb.set_style(
        ProgressStyle::with_template("{spinner:.183} {msg}")
            .unwrap()
            .tick_strings(&[
                "━         ",
                "━━        ",
                "━━━       ",
                "━━━━      ",
                "━━━━━     ",
                "━━━━━━    ",
                "━━━━━━━   ",
                "━━━━━━━━  ",
                "━━━━━━━━━ ",
                "━━━━━━━━━━",
            ]),
    );
    pb
}
