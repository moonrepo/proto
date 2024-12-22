use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use miette::IntoDiagnostic;
use semver::Version;
use starbase_console::ui::{style_to_color, ConsoleTheme, Style};
use starbase_styles::color;
use starbase_utils::env::bool_var;
use std::io::IsTerminal;
use tracing::debug;

pub fn create_console_theme() -> ConsoleTheme {
    let mut theme = ConsoleTheme::branded(style_to_color(Style::Shell));
    let mut frames = vec![];

    for i in 1..=20 {
        if i == 20 {
            frames.push("━".repeat(20));
        } else {
            frames.push(format!("{}╾{}", "━".repeat(i - 1), "─".repeat(20 - i)));
        }
    }

    theme.progress_loader_frames = frames;
    theme.progress_bar_filled_char = '━';
    theme.progress_bar_unfilled_char = '─';
    theme.progress_bar_position_char = '╾';
    theme
}

fn format_template_styles(template: &str) -> String {
    let pipe = color::muted(" | ");
    let slash = color::muted(" / ");

    template.replace(" | ", &pipe).replace(" / ", &slash)
}

pub fn create_progress_bar_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .progress_chars("━╾─")
        .template(format_template_styles("{prefix} {bar:20.183/239} | {msg}").as_str())
        .unwrap()
}

pub fn create_progress_bar_download_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .progress_chars("━╾─")
        .template(format_template_styles("{prefix} {bar:20.183/239} | {bytes:>5.248} / {total_bytes:5.248} | {bytes_per_sec:>5.183} | {msg}").as_str())
        .unwrap()
}

pub fn create_progress_spinner_style() -> ProgressStyle {
    let mut frames = vec![];

    for i in 1..=20 {
        if i == 20 {
            frames.push("━".repeat(20));
        } else {
            frames.push(format!("{}╾{}", "━".repeat(i - 1), "─".repeat(20 - i)));
        }
    }

    let frames = frames.iter().map(|f| f.as_str()).collect::<Vec<_>>();

    ProgressStyle::default_spinner()
        .tick_strings(&frames)
        .template(format_template_styles("{prefix} {spinner:20.183/239} | {msg}").as_str())
        .unwrap()
}

fn is_hidden_progress() -> bool {
    bool_var("PROTO_NO_PROGRESS") || !std::io::stderr().is_terminal()
}

pub fn create_multi_progress_bar() -> MultiProgress {
    if is_hidden_progress() {
        MultiProgress::with_draw_target(ProgressDrawTarget::hidden())
    } else {
        MultiProgress::new()
    }
}

pub fn create_progress_bar<S: AsRef<str>>(start: S) -> ProgressBar {
    let pb = if is_hidden_progress() {
        ProgressBar::hidden()
    } else {
        ProgressBar::new(0)
    };

    pb.set_style(create_progress_bar_style());
    pb.set_position(0);
    pb.set_length(100);

    print_progress_state(&pb, start.as_ref().to_owned());

    pb
}

// When not a TTY, we should display something to the user!
pub fn print_progress_state(pb: &ProgressBar, message: String) {
    if message.is_empty() || pb.message() == message {
        return;
    }

    pb.set_message(message);

    if pb.is_hidden() {
        // This expands tokens, so don't use the argument message!
        println!("{} {}", pb.prefix(), pb.message());
    }
}

pub async fn fetch_latest_version() -> miette::Result<Version> {
    let version = reqwest::get("https://raw.githubusercontent.com/moonrepo/proto/master/version")
        .await
        .into_diagnostic()?
        .text()
        .await
        .into_diagnostic()?
        .trim()
        .to_string();

    debug!("Found latest version {}", color::hash(&version));

    Ok(Version::parse(&version).unwrap())
}
