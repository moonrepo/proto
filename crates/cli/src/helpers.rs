use clap::ValueEnum;
use dialoguer::{
    console::{style, Style},
    theme::ColorfulTheme,
};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use miette::IntoDiagnostic;
use proto_core::PinType;
use starbase_styles::color::{self, Color};
use starbase_utils::env::bool_var;
use std::{io::IsTerminal, time::Duration};
use tracing::debug;

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum PinOption {
    #[value(alias = "store")]
    Global,
    #[default]
    #[value(alias = "cwd")]
    Local,
    #[value(alias = "home")]
    User,
}

pub fn map_pin_type(global: bool, option: Option<PinOption>) -> PinType {
    if let Some(option) = option {
        return match option {
            PinOption::Global => PinType::Global,
            PinOption::Local => PinType::Local,
            PinOption::User => PinType::User,
        };
    }

    if global {
        PinType::Global
    } else {
        PinType::Local
    }
}

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

pub fn create_progress_spinner<S: AsRef<str>>(start: S) -> ProgressBar {
    let pb = if is_hidden_progress() {
        ProgressBar::hidden()
    } else {
        ProgressBar::new_spinner()
    };

    pb.set_style(create_progress_spinner_style());
    pb.enable_steady_tick(Duration::from_millis(100));

    print_progress_state(&pb, start.as_ref().to_owned());

    pb
}

// When not a TTY, we should display something to the user!
pub fn print_progress_state(pb: &ProgressBar, message: String) {
    if message.is_empty() {
        return;
    }

    pb.set_message(message);

    if pb.is_hidden() {
        // This expands tokens, so don't use the argument message!
        println!("{} {}", pb.prefix(), pb.message());
    }
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
