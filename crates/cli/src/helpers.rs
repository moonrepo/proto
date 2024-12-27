use miette::IntoDiagnostic;
use semver::Version;
use starbase_console::ui::{style_to_color, ConsoleTheme, Style};
use starbase_styles::color;
use tracing::debug;

pub fn create_console_theme() -> ConsoleTheme {
    let mut theme = ConsoleTheme::branded(style_to_color(Style::Shell));
    let mut frames = vec![];

    for i in 1..=20 {
        if i == 20 {
            frames.push("━".repeat(20));
        } else {
            frames.push(format!("{}╾{}", "━".repeat(i - 1), " ".repeat(20 - i)));
        }
    }

    theme.progress_loader_frames = frames;
    theme.progress_bar_filled_char = '━';
    theme.progress_bar_unfilled_char = '─';
    theme.progress_bar_position_char = '╾';

    theme
        .custom_tags
        .insert("version".into(), style_to_color(Style::Success));
    theme
        .custom_tags
        .insert("versionalt".into(), style_to_color(Style::Invalid));
    theme
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
