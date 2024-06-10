use crate::error::ProtoCliError;
use crate::helpers::fetch_latest_version;
use crate::session::ProtoSession;
use crate::telemetry::{track_usage, Metric};
use indicatif::{ProgressBar, ProgressStyle};
use proto_core::is_offline;
use proto_installer::{determine_triple, download_release, unpack_release};
use semver::Version;
use starbase::AppResult;
use starbase_styles::color;
use tracing::{debug, trace};

#[tracing::instrument(skip_all)]
pub async fn upgrade(session: ProtoSession) -> AppResult {
    if is_offline() {
        return Err(ProtoCliError::UpgradeRequiresInternet.into());
    }

    let current_version = &session.cli_version;
    let latest_version = fetch_latest_version().await?;

    debug!(
        "Comparing latest version {} to current version {}",
        color::hash(&latest_version),
        color::hash(current_version),
    );

    if Version::parse(&latest_version).unwrap() <= Version::parse(current_version).unwrap() {
        println!("You're already on the latest version of proto!");

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
        &session.env.store.temp_dir,
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

    let upgraded = unpack_release(
        result,
        session.env.store.bin_dir.clone(),
        session
            .env
            .store
            .inventory_dir
            .join("proto")
            .join(current_version),
        true,
    )?;

    // Track usage metrics
    track_usage(
        &session.env,
        Metric::UpgradeProto {
            old_version: current_version.to_owned(),
            new_version: latest_version.to_owned(),
        },
    )
    .await?;

    if upgraded {
        println!("Upgraded proto to v{}!", latest_version);

        return Ok(());
    }

    Err(ProtoCliError::UpgradeFailed {
        bin: "proto".into(),
    }
    .into())
}
