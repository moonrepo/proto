use crate::helpers::download_to_temp_with_progress_bar;
use miette::IntoDiagnostic;
use proto_core::{get_bin_dir, get_temp_dir, is_offline, ProtoError};
use semver::Version;
use starbase::system;
use starbase_archive::Archiver;
use starbase_styles::color;
use starbase_utils::fs;
use std::env::consts;
use std::path::PathBuf;
use tracing::{debug, info, warn};

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
pub async fn upgrade() {
    if is_offline() {
        return Err(ProtoError::Message(
            "Upgrading proto requires an internet connection!".into(),
        ))?;
    }

    let version = env!("CARGO_PKG_VERSION");
    let new_version = fetch_version().await?;

    debug!(
        "Comparing latest version {} to local version {}",
        color::hash(&new_version),
        color::hash(version),
    );

    if Version::parse(&new_version).unwrap() <= Version::parse(version).unwrap() {
        info!("You're already on the latest version of proto!");

        return Ok(());
    }

    // Determine the download file based on target
    let target = match (consts::OS, consts::ARCH) {
        ("linux", arch) => format!("{arch}-unknown-linux-gnu"),
        ("macos", arch) => format!("{arch}-apple-darwin"),
        ("windows", "x86_64") => "x86_64-pc-windows-msvc".to_owned(),
        (os, arch) => {
            return Err(ProtoError::Message(format!(
                "Unable to upgrade proto, unsupported platform {} + {}.",
                os, arch
            )))?;
        }
    };
    let target_ext = if cfg!(windows) { "zip" } else { "tar.xz" };
    let target_file = format!("proto_cli-{target}");

    debug!("Download target: {}", &target_file);

    // Download the file and show a progress bar
    let download_file = format!("{target_file}.{target_ext}");
    let download_url = format!(
        "https://github.com/moonrepo/proto/releases/download/v{new_version}/{download_file}"
    );
    let temp_file = download_to_temp_with_progress_bar(&download_url, &download_file).await?;
    let temp_dir = get_temp_dir()?.join(&target_file);

    // Unpack the downloaded file
    Archiver::new(&temp_dir, &temp_file).unpack_from_ext()?;

    // Move the old binary
    let bin_dir = get_bin_dir()?;
    let bin_name = if cfg!(windows) { "proto.exe" } else { "proto" };
    let bin_path = bin_dir.join(bin_name);

    if bin_path.exists() {
        fs::rename(
            &bin_path,
            bin_dir.join(if cfg!(windows) {
                "proto-old.exe"
            } else {
                "proto-old"
            }),
        )?;
    }

    // Move the new binary to the bins directory
    let lookup_paths = vec![
        PathBuf::from(target_file).join(bin_name),
        PathBuf::from(bin_name),
    ];

    for lookup_path in lookup_paths {
        let possible_bin_path = temp_dir.join(lookup_path);

        if possible_bin_path.exists() {
            fs::copy_file(possible_bin_path, &bin_path)?;
            fs::update_perms(&bin_path, None)?;

            fs::remove(temp_dir)?;
            fs::remove(temp_file)?;

            info!("Upgraded proto to v{}!", new_version);
            warn!("Changes to PATH were made in v0.20. Please refer to the changelog and migration guide!");

            return Ok(());
        }
    }

    Err(ProtoError::Message(format!(
        "Failed to upgrade proto, {} could not be located after download!",
        color::shell(bin_name)
    )))?
}
