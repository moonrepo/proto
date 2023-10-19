use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use miette::IntoDiagnostic;
use proto_core::{
    get_temp_dir, load_tool_from_locator, Id, ProtoEnvironment, ProtoError, Tool, ToolsConfig,
};
use starbase_utils::fs;
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub fn enable_progress_bars() {
    env::remove_var("PROTO_NO_PROGRESS");
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
) -> miette::Result<PathBuf> {
    let handle_error = |error: reqwest::Error| ProtoError::Http {
        url: url.to_owned(),
        error,
    };
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
    let mut file = fs::create_file(&temp_file)?;
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

pub async fn load_configured_tools() -> miette::Result<Vec<Tool>> {
    load_configured_tools_with_filters(HashSet::new()).await
}

pub async fn load_configured_tools_with_filters(filter: HashSet<&Id>) -> miette::Result<Vec<Tool>> {
    let proto = Arc::new(ProtoEnvironment::new()?);
    let user_config = Arc::new(proto.get_user_config()?);

    let mut tools_config = ToolsConfig::load_upwards_from(&proto.cwd, false)?;
    tools_config.inherit_builtin_plugins();

    let mut plugins = HashMap::new();
    plugins.extend(user_config.plugins.clone());
    plugins.extend(tools_config.plugins);

    let mut futures = vec![];
    let mut tools = vec![];

    for (id, locator) in plugins {
        if !filter.is_empty() && !filter.contains(&id) {
            continue;
        }

        let proto = Arc::clone(&proto);
        let user_config = Arc::clone(&user_config);

        futures.push(tokio::spawn(async move {
            load_tool_from_locator(id, proto, locator, &user_config).await
        }));
    }

    for future in futures {
        tools.push(future.await.into_diagnostic()??);
    }

    Ok(tools)
}
