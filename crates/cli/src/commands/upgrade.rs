use crate::helpers::download_to_temp_with_progress_bar;
use proto_core::{color, get_bin_dir, get_temp_dir, is_offline, load_git_tags, unpack, ProtoError};
use semver::Version;
use starbase::SystemResult;
use starbase_utils::fs;
use std::env::consts;
use tracing::{debug, info};

async fn fetch_version() -> Result<String, ProtoError> {
    let tags = load_git_tags("https://github.com/moonrepo/proto")
        .await?
        .into_iter()
        .filter(|t| t.starts_with('v'))
        .collect::<Vec<_>>();

    let latest = tags.last().unwrap().strip_prefix('v').unwrap().to_owned();

    debug!("Found latest version {}", color::id(&latest));

    Ok(latest)
}

pub async fn upgrade() -> SystemResult {
    if is_offline() {
        return Err(ProtoError::Message(
            "Upgrading proto requires an internet connection!".into(),
        ))?;
    }

    let version = env!("CARGO_PKG_VERSION");
    let new_version = fetch_version().await?;

    debug!(
        "Comparing latest version {} to local version {}",
        color::id(&new_version),
        color::id(version),
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
        (_, arch) => {
            return Err(ProtoError::UnsupportedArchitecture(
                "proto".to_owned(),
                arch.to_owned(),
            ))?;
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
    let temp_dir = get_temp_dir()?;

    // Unpack the downloaded file
    unpack(temp_file, &temp_dir, None)?;

    // Move the new binary to the bins directory
    let bin_name = if cfg!(windows) { "proto.exe" } else { "proto" };
    let bin_path = get_bin_dir()?.join(bin_name);

    fs::copy_file(temp_dir.join(target_file).join(bin_name), &bin_path)?;
    fs::update_perms(&bin_path, None)?;

    info!("Upgraded proto to v{}!", new_version);

    Ok(())
}
