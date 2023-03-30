use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use proto::{get_temp_dir, ProtoError};
use std::cmp;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
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

pub fn disable_progress_bars() {
    env::set_var("PROTO_NO_PROGRESS", "1");
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

pub async fn download_to_temp_with_progress_bar(
    url: &str,
    file_name: &str,
) -> Result<PathBuf, ProtoError> {
    let handle_error = |e: reqwest::Error| ProtoError::Http(url.to_owned(), e.to_string());
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(handle_error)?;
    let total_size = response.content_length().unwrap_or(0);

    // Create progress bar
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar().progress_chars("━╾─").template(
        "{bar:80.183/black} | {bytes:.239} / {total_bytes:.248} | {bytes_per_sec:.183} | eta {eta}",
    ).unwrap());

    // Download in chunks
    let temp_file = get_temp_dir()?.join(file_name);
    let mut file = fs::File::create(&temp_file)
        .map_err(|e| ProtoError::Fs(temp_file.clone(), e.to_string()))?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(item) = stream.next().await {
        let chunk = item.unwrap();
        file.write_all(&chunk).unwrap();
        let new = cmp::min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_and_clear();

    Ok(temp_file)
}
