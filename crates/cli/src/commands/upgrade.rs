use crate::error::ProtoCliError;
use crate::helpers::ProtoResource;
use crate::telemetry::{track_usage, Metric};
use indicatif::{ProgressBar, ProgressStyle};
use miette::IntoDiagnostic;
use proto_core::is_offline;
use proto_installer::{determine_triple, download_release};
use semver::Version;
use starbase::system;
use starbase_archive::Archiver;
use starbase_styles::color;
use starbase_utils::fs;
use std::env::{self, consts};
use std::path::PathBuf;
use tracing::{debug, info, trace};

async fn fetch_version() -> miette::Result<String> {
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

pub fn is_musl() -> bool {
    let Ok(output) = std::process::Command::new("ldd").arg("--version").output() else {
        return false;
    };

    String::from_utf8(output.stdout).map_or(false, |out| out.contains("musl"))
}

#[system]
pub async fn upgrade(proto: ResourceRef<ProtoResource>) {
    if is_offline() {
        return Err(ProtoCliError::UpgradeRequiresInternet.into());
    }

    let current_version = env!("CARGO_PKG_VERSION");
    let latest_version = fetch_version().await?;

    debug!(
        "Comparing latest version {} to current version {}",
        color::hash(&latest_version),
        color::hash(current_version),
    );

    if Version::parse(&latest_version).unwrap() <= Version::parse(current_version).unwrap() {
        info!("You're already on the latest version of proto!");

        return Ok(());
    }

    // Determine the download file based on target
    let triple_target = determine_triple()?;

    debug!("Download target: {}", triple_target);

    // Download the file and show a progress bar
    let pb = ProgressBar::new(0);
    pb.set_style(ProgressStyle::default_bar().progress_chars("━╾─").template(
        "{bar:80.183/black} | {bytes:.239} / {total_bytes:.248} | {bytes_per_sec:.183} | eta {eta}",
    ).unwrap());

    let archive_file = download_release(
        &triple_target,
        &latest_version,
        &proto.env.temp_dir,
        |downloaded_size, total_size| {
            if downloaded_size == 0 {
                pb.set_length(total_size);
            } else {
                pb.set_position(downloaded_size);
            }

            trace!("Downloaded {} of {} bytes", downloaded_size, total_size);
        },
    )
    .await?;

    pb.finish_and_clear();

    // Unpack the downloaded file
    // Archiver::new(&temp_dir, &temp_file).unpack_from_ext()?;

    // // Move the old binaries
    // let bin_names = if cfg!(windows) {
    //     vec!["proto.exe", "proto-shim.exe"]
    // } else {
    //     vec!["proto", "proto-shim"]
    // };
    // let bin_dir = match env::var("PROTO_INSTALL_DIR") {
    //     Ok(dir) => PathBuf::from(dir),
    //     Err(_) => proto.env.bin_dir.clone(),
    // };

    // for bin_name in &bin_names {
    //     let output_path = bin_dir.join(bin_name);
    //     let relocate_path = proto
    //         .env
    //         .tools_dir
    //         .join("proto")
    //         .join(current_version)
    //         .join(bin_name);

    //     if output_path.exists() {
    //         fs::rename(&output_path, &relocate_path)?;
    //     }

    //     // If not installed at our standard location
    //     if let Ok(current_exe) = env::current_exe() {
    //         if current_exe != output_path
    //             && current_exe
    //                 .file_name()
    //                 .is_some_and(|name| name == *bin_name)
    //         {
    //             fs::rename(&current_exe, &relocate_path)?;
    //         }
    //     }
    // }

    // // Move the new binary to the bins directory
    // let mut upgraded = false;

    // for bin_name in &bin_names {
    //     let output_path = bin_dir.join(bin_name);
    //     let input_paths = vec![
    //         temp_dir.join(&target_file).join(bin_name),
    //         temp_dir.join(bin_name),
    //     ];

    //     for input_path in input_paths {
    //         if input_path.exists() {
    //             fs::copy_file(input_path, &output_path)?;
    //             fs::update_perms(&output_path, None)?;

    //             upgraded = true;
    //             break;
    //         }
    //     }
    // }

    // fs::remove(temp_dir)?;
    // fs::remove(temp_file)?;

    // Track usage metrics
    track_usage(
        &proto.env,
        Metric::UpgradeProto {
            old_version: current_version.to_owned(),
            new_version: latest_version.to_owned(),
        },
    )
    .await?;

    // if upgraded {
    //     info!("Upgraded proto to v{}!", latest_version);

    //     return Ok(());
    // }

    // Err(ProtoCliError::UpgradeFailed {
    //     bin: bin_names[0].to_owned(),
    // })?;
}
