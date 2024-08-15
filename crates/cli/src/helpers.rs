use dialoguer::{
    console::{style, Style},
    theme::ColorfulTheme,
};
use indicatif::{ProgressBar, ProgressStyle};
use miette::IntoDiagnostic;
use starbase_styles::color::{self, Color};
use starbase_utils::env::bool_var;
use std::env;
use std::time::Duration;
use tracing::debug;

pub fn create_theme() -> ColorfulTheme {
    ColorfulTheme {
        defaults_style: Style::new().for_stderr().color256(Color::Pink as u8),
        prompt_style: Style::new().for_stderr(),
        prompt_prefix: style("?".to_string())
            .for_stderr()
            .color256(Color::Blue as u8),
        prompt_suffix: style("›".to_string())
            .for_stderr()
            .color256(Color::Gray as u8),
        success_prefix: style("✔".to_string())
            .for_stderr()
            .color256(Color::Green as u8),
        success_suffix: style("·".to_string())
            .for_stderr()
            .color256(Color::Gray as u8),
        error_prefix: style("✘".to_string())
            .for_stderr()
            .color256(Color::Red as u8),
        error_style: Style::new().for_stderr().color256(Color::Pink as u8),
        hint_style: Style::new().for_stderr().color256(Color::Purple as u8),
        values_style: Style::new().for_stderr().color256(Color::Purple as u8),
        active_item_style: Style::new().for_stderr().color256(Color::Teal as u8),
        inactive_item_style: Style::new().for_stderr(),
        active_item_prefix: style("❯".to_string())
            .for_stderr()
            .color256(Color::Teal as u8),
        inactive_item_prefix: style(" ".to_string()).for_stderr(),
        checked_item_prefix: style("✔".to_string())
            .for_stderr()
            .color256(Color::Teal as u8),
        unchecked_item_prefix: style("✔".to_string())
            .for_stderr()
            .color256(Color::GrayLight as u8),
        picked_item_prefix: style("❯".to_string())
            .for_stderr()
            .color256(Color::Teal as u8),
        unpicked_item_prefix: style(" ".to_string()).for_stderr(),
    }
}

// pub fn enable_progress_bars() {
//     env::remove_var("PROTO_NO_PROGRESS");
// }

pub fn disable_progress_bars() {
    env::set_var("PROTO_NO_PROGRESS", "1");
}

pub fn create_progress_bar_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .progress_chars("━╾─")
        .template("{prefix} {bar:20.183/239} | {msg}")
        .unwrap()
}

pub fn create_progress_bar_download_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .progress_chars("━╾─")
        .template("{prefix} {bar:20.183/239} | {bytes:>5.248} / {total_bytes:5.248} | {bytes_per_sec:>5.183} | {msg}")
        .unwrap()
}

pub fn create_progress_spinner_style() -> ProgressStyle {
    let mut chars = vec![];

    for i in 1..=20 {
        if i == 20 {
            chars.push("━".repeat(20));
        } else {
            chars.push(format!("{}╾{}", "━".repeat(i - 1), " ".repeat(20 - i)));
        }
    }

    let chars = chars.iter().map(|c| c.as_str()).collect::<Vec<_>>();

    ProgressStyle::default_spinner()
        .tick_strings(&chars)
        .template("{prefix} {spinner:20.183/239} | {msg}")
        .unwrap()
}

pub fn create_progress_bar<S: AsRef<str>>(start: S) -> ProgressBar {
    let pb = if bool_var("PROTO_NO_PROGRESS") {
        ProgressBar::hidden()
    } else {
        ProgressBar::new(0)
    };

    pb.set_style(create_progress_bar_style());
    pb.set_message(start.as_ref().to_owned());
    pb.set_position(0);
    pb.set_length(100);
    pb
}

pub fn create_progress_spinner<S: AsRef<str>>(start: S) -> ProgressBar {
    let pb = if bool_var("PROTO_NO_PROGRESS") {
        ProgressBar::hidden()
    } else {
        ProgressBar::new_spinner()
    };

    pb.set_style(create_progress_spinner_style());
    pb.set_message(start.as_ref().to_owned());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

pub async fn fetch_latest_version() -> miette::Result<String> {
    let version = reqwest::get("https://raw.githubusercontent.com/moonrepo/proto/master/version")
        .await
        .into_diagnostic()?
        .text()
        .await
        .into_diagnostic()?
        .trim()
        .to_string();

    debug!("Found latest version {}", color::hash(&version));

    Ok(version)
}
