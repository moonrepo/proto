use std::env;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

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
