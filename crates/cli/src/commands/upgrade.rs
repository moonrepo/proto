use crate::error::ProtoCliError;
use crate::helpers::ProtoResource;
use crate::telemetry::{track_usage, Metric};
use indicatif::{ProgressBar, ProgressStyle};
use miette::IntoDiagnostic;
use proto_core::is_offline;
use proto_installer::{determine_triple, download_release, unpack_release};
use semver::Version;
use starbase::system;
use starbase_styles::color;
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

    let result = download_release(
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
    debug!(archive = ?result.archive_file, "Unpacking download");

    let upgraded = unpack_release(result, &proto.env.bin_dir, &proto.env.tools_dir)?;

    // Track usage metrics
    track_usage(
        &proto.env,
        Metric::UpgradeProto {
            old_version: current_version.to_owned(),
            new_version: latest_version.to_owned(),
        },
    )
    .await?;

    if upgraded {
        info!("Upgraded proto to v{}!", latest_version);

        return Ok(());
    }

    Err(ProtoCliError::UpgradeFailed {
        bin: "proto".into(),
    })?;
}
