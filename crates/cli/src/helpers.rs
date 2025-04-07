use crate::error::ProtoCliError;
use miette::IntoDiagnostic;
use semver::Version;
use starbase_console::ui::{ConsoleTheme, Style, style_to_color};
use starbase_styles::color;
use starbase_utils::json::JsonValue;
use std::env;
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

async fn fetch_from_github(url: &str) -> reqwest::Result<reqwest::Response> {
    let mut request = reqwest::Client::new()
        .get(url)
        .header("User-Agent", "moonrepo-proto-cli");

    if let Ok(token) = env::var("PROTO_GITHUB_TOKEN") {
        request = request.header("Authorization", format!("Bearer {token}"));
    }

    request.send().await
}

async fn inner_fetch_latest_version() -> reqwest::Result<Option<String>> {
    let response =
        fetch_from_github("https://api.github.com/repos/moonrepo/proto/releases/latest").await?;

    if response.status().is_success() {
        let release: JsonValue = response.json().await?;

        if let Some(JsonValue::String(tag)) = release.get("tag_name") {
            return Ok(Some(tag.trim_start_matches('v').to_string()));
        }
    }

    // Tag field doesn't exist, or the request failed,
    // or the data is incomplete, so fallback
    let response =
        fetch_from_github("https://raw.githubusercontent.com/moonrepo/proto/master/version")
            .await?;

    if response.status().is_success() {
        return Ok(Some(response.text().await?.trim().to_string()));
    }

    Ok(None)
}

pub async fn fetch_latest_version() -> miette::Result<Version> {
    let Some(version) = inner_fetch_latest_version().await.into_diagnostic()? else {
        return Err(ProtoCliError::FailedToFetchVersion.into());
    };

    debug!("Found latest version {}", color::hash(&version));

    Version::parse(&version).into_diagnostic()
}
