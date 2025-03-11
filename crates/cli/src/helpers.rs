use miette::IntoDiagnostic;
use semver::Version;
use starbase_console::ui::{ConsoleTheme, Style, style_to_color};
use starbase_styles::color;
use starbase_utils::json::JsonValue;
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
    let client = reqwest::Client::new();
    let release: JsonValue = client
        .get("https://api.github.com/repos/moonrepo/proto/releases/latest")
        .header("User-Agent", "moonrepo-proto-cli")
        .send()
        .await
        .into_diagnostic()?
        .json()
        .await
        .into_diagnostic()?;

    let version = release["tag_name"]
        .as_str()
        .ok_or_else(|| miette::miette!("Invalid release format: {}", release["tag_name"]))?
        .trim_start_matches('v')
        .to_string();

    debug!("Found latest version {}", color::hash(&version));

    Version::parse(&version).into_diagnostic()
}
