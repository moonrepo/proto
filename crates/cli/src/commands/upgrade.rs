use crate::error::ProtoCliError;
use crate::helpers::{download_to_temp_with_progress_bar, ProtoResource};
use miette::IntoDiagnostic;
use proto_core::is_offline;
use semver::Version;
use starbase::system;
use starbase_archive::Archiver;
use starbase_styles::color;
use starbase_utils::fs;
use std::env::{self, consts};
use std::path::PathBuf;
use tracing::{debug, info};

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
        "Comparing latest version {} to local version {}",
        color::hash(&latest_version),
        color::hash(current_version),
    );

    if Version::parse(&latest_version).unwrap() <= Version::parse(current_version).unwrap() {
        info!("You're already on the latest version of proto!");

        return Ok(());
    }

    // Determine the download file based on target
    let target = match (consts::OS, consts::ARCH) {
        ("linux", arch) => format!(
            "{arch}-unknown-linux-{}",
            if is_musl() { "musl" } else { "gnu" }
        ),
        ("macos", arch) => format!("{arch}-apple-darwin"),
        ("windows", "x86_64") => "x86_64-pc-windows-msvc".to_owned(),
        (os, arch) => {
            return Err(ProtoCliError::UpgradeInvalidPlatform {
                arch: arch.to_owned(),
                os: os.to_owned(),
            }
            .into());
        }
    };
    let target_ext = if cfg!(windows) { "zip" } else { "tar.xz" };
    let target_file = format!("proto_cli-{target}");

    debug!("Download target: {}", &target_file);

    // Download the file and show a progress bar
    let download_file = format!("{target_file}.{target_ext}");
    let download_url = format!(
        "https://github.com/moonrepo/proto/releases/download/v{latest_version}/{download_file}"
    );
    let temp_file = download_to_temp_with_progress_bar(&download_url, &download_file).await?;
    let temp_dir = proto.env.temp_dir.join(&target_file);

    // Unpack the downloaded file
    Archiver::new(&temp_dir, &temp_file).unpack_from_ext()?;

    // Move the old binaries
    let bin_names = if cfg!(windows) {
        vec!["proto.exe", "proto-shim.exe"]
    } else {
        vec!["proto", "proto-shim"]
    };
    let bin_dir = match env::var("PROTO_INSTALL_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => proto.env.bin_dir.clone(),
    };

    for bin_name in &bin_names {
        let output_path = bin_dir.join(bin_name);
        let relocate_path = proto
            .env
            .tools_dir
            .join("proto")
            .join(current_version)
            .join(bin_name);

        if output_path.exists() {
            fs::rename(&output_path, &relocate_path)?;
        }

        if let Ok(current_exe) = env::current_exe() {
            if bin_name == &bin_names[0] && current_exe != output_path {
                fs::rename(&current_exe, &relocate_path)?;
            }
        }
    }

    // Move the new binary to the bins directory
    let mut upgraded = false;

    for bin_name in &bin_names {
        let output_path = bin_dir.join(bin_name);
        let input_paths = vec![
            temp_dir.join(&target_file).join(bin_name),
            temp_dir.join(bin_name),
        ];

        for input_path in input_paths {
            if input_path.exists() {
                fs::copy_file(input_path, &output_path)?;
                fs::update_perms(&output_path, None)?;
                upgraded = true;
            }
        }
    }

    fs::remove(temp_dir)?;
    fs::remove(temp_file)?;

    if upgraded {
        info!("Upgraded proto to v{}!", latest_version);

        return Ok(());
    }

    Err(ProtoCliError::UpgradeFailed {
        bin: bin_names[0].to_owned(),
    })?;
}
