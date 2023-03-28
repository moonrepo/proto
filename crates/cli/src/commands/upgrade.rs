use crate::helpers::enable_logging;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info};
use proto::get_temp_dir;
use proto_core::{color, is_offline, ProtoError};
use semver::Version;
use serde::Deserialize;
use std::cmp::min;
use std::io::Write;
use std::{
    env::{self, consts},
    fs::File,
};

#[derive(Deserialize)]
struct Meta {
    #[serde(rename = "crate")]
    crate_data: MetaCrate,
}

#[derive(Deserialize)]
struct MetaCrate {
    newest_version: String,
}

async fn fetch_version() -> Result<String, ProtoError> {
    let url = "https://crates.io/api/v1/crates/proto_cli";

    debug!(
        target: "proto:upgrade",
        "Fetching latest version from {}",
        color::url(&url),
    );

    let res = reqwest::get(url)
        .await
        .map_err(|e| ProtoError::Http(url.to_owned(), e.to_string()))?
        .json::<Meta>()
        .await
        .map_err(|e| ProtoError::Http(url.to_owned(), e.to_string()))?;

    debug!(
        target: "proto:upgrade",
        "Found latest version {}",
        color::id(&res.crate_data.newest_version),
    );

    Ok(res.crate_data.newest_version)
}

pub async fn upgrade() -> Result<(), ProtoError> {
    enable_logging();

    if is_offline() {
        return Err(ProtoError::Message(
            "Upgrading proto requires an internet connection!".into(),
        ));
    }

    let version = env!("CARGO_PKG_VERSION");
    let new_version = fetch_version().await?;

    if Version::parse(&new_version).unwrap() <= Version::parse(version).unwrap() {
        println!("You're already on the latest version of proto!");

        return Ok(());
    }

    // Determine the download file based on target
    let target = match (consts::OS, consts::ARCH) {
        ("linux", arch) => format!("{arch}-unknown-linux-gnu"),
        ("macos", arch) => format!("{arch}-apple-darwin"),
        ("windows", "x86_64") => "x86_64-pc-windows-msvc".to_owned(),
        (_, arch) => {
            return Err(
                ProtoError::UnsupportedArchitecture("proto".to_owned(), arch.to_owned()).into(),
            );
        }
    };
    let target_file = format!(
        "proto_cli-v{new_version}-{target}.{}",
        if consts::OS == "windows" {
            "zip"
        } else {
            "tar.xz"
        }
    );

    debug!(
        target: "proto:upgrade",
        "Target: {}",
        &target,
    );

    debug!(
        target: "proto:upgrade",
        "Target file name: {}",
        &target_file,
    );

    let current_bin_path = env::current_exe().expect("TODO");

    // Download the file and show a progress bar
    let file_download_url =
        format!("https://github.com/moonrepo/proto/releases/download/v{new_version}/{target_file}");
    let response = reqwest::get(&file_download_url)
        .await
        .map_err(|e| ProtoError::Http(file_download_url.to_owned(), e.to_string()))?;
    let file_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(file_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").unwrap()
        .progress_chars("#>-"));
    pb.set_message(format!("Downloading proto v{new_version}..."));

    let temp_file_path = get_temp_dir()?.join(&target_file);
    let mut temp_file = File::create(&temp_file_path).expect("TODO");
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(item) = stream.next().await {
        let chunk = item.unwrap();
        temp_file.write_all(&chunk).unwrap();

        let progress = min(downloaded + (chunk.len() as u64), file_size);
        downloaded = progress;
        pb.set_position(progress);
    }

    pb.finish_with_message("Download complete!");

    Ok(())
}
