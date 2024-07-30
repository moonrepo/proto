use crate::error::ProtoCliError;
use crate::helpers::fetch_latest_version;
use crate::session::ProtoSession;
use crate::telemetry::{track_usage, Metric};
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use proto_core::is_offline;
use proto_installer::{determine_triple, download_release, unpack_release};
use semver::Version;
use starbase::AppResult;
use starbase_styles::color;
use tracing::{debug, trace};

#[derive(Args, Clone, Debug)]
pub struct UpgradeArgs {
    #[arg(help = "Explicit version to upgrade or downgrade to")]
    pub version: Option<Version>,
}

#[tracing::instrument(skip_all)]
pub async fn upgrade(session: ProtoSession, args: UpgradeArgs) -> AppResult {
    if is_offline() {
        return Err(ProtoCliError::UpgradeRequiresInternet.into());
    }

    let explicit_upgrade = args.version.is_some();

    let current_version = Version::parse(&session.cli_version).unwrap();
    let latest_version = fetch_latest_version().await?;
    let target_version = match args.version {
        Some(version) => version,
        None => Version::parse(&latest_version).unwrap(),
    };

    debug!(
        "Comparing target version {} to current version {}",
        color::hash(target_version.to_string()),
        color::hash(current_version.to_string()),
    );

    if !explicit_upgrade && target_version <= current_version || target_version == current_version {
        println!("You're already on version {} of proto!", current_version);

        return Ok(());
    }

    // Determine the download file based on target
    let target_triple = determine_triple()?;

    debug!("Download target: {}", target_triple);

    // Download the file and show a progress bar
    let pb = ProgressBar::new(0);
    pb.set_style(ProgressStyle::default_bar().progress_chars("━╾─").template(
        "{bar:80.183/black} | {bytes:.239} / {total_bytes:.248} | {bytes_per_sec:.183} | eta {eta}",
    ).unwrap());

    let result = download_release(
        &target_triple,
        &target_version.to_string(),
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

    let unpacked = unpack_release(
        result,
        &session.env.store.bin_dir,
        session
            .env
            .store
            .inventory_dir
            .join("proto")
            .join(current_version.to_string()),
        true,
    )?;

    // Track usage metrics
    track_usage(
        &session.env,
        Metric::UpgradeProto {
            old_version: current_version.to_string(),
            new_version: target_version.to_string(),
        },
    )
    .await?;

    if unpacked {
        #[allow(clippy::comparison_chain)]
        if target_version > current_version {
            println!("Upgraded proto to v{}!", target_version);
        } else if target_version < current_version {
            println!("Downgraded proto to v{}!", target_version);
        }

        return Ok(());
    }

    Err(ProtoCliError::UpgradeFailed {
        bin: "proto".into(),
    }
    .into())
}
