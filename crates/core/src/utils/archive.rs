use super::process::{ProtoProcessError, exec_command_piped, handle_exec};
use proto_pdk_api::ArchiveSource;
use starbase_archive::{ArchiveError, Archiver};
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::net::{DownloadOptions, NetError};
use starbase_utils::{fs, net};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::process::Command;
use warpgate::extract_file_name_from_url;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoArchiveError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Archive(#[from] Box<ArchiveError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Net(#[from] Box<NetError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Process(#[from] Box<ProtoProcessError>),

    #[diagnostic(code(proto::archive::missing_pkg_payload))]
    #[error("Unable to find a payload in macOS package {}.", .path.style(Style::Path))]
    MissingPkgPayload { path: PathBuf },

    #[diagnostic(code(proto::archive::missing_pkg_contents))]
    #[error(
        "Unable to extract contents from macOS package {}, using directory prefix {}.",
        .path.style(Style::Path),
        .prefix.style(Style::Label)
    )]
    MissingPkgContents { path: PathBuf, prefix: String },
}

impl From<ArchiveError> for ProtoArchiveError {
    fn from(e: ArchiveError) -> ProtoArchiveError {
        ProtoArchiveError::Archive(Box::new(e))
    }
}

impl From<FsError> for ProtoArchiveError {
    fn from(e: FsError) -> ProtoArchiveError {
        ProtoArchiveError::Fs(Box::new(e))
    }
}

impl From<NetError> for ProtoArchiveError {
    fn from(e: NetError) -> ProtoArchiveError {
        ProtoArchiveError::Net(Box::new(e))
    }
}

impl From<ProtoProcessError> for ProtoArchiveError {
    fn from(error: ProtoProcessError) -> ProtoArchiveError {
        ProtoArchiveError::Process(Box::new(error))
    }
}

pub fn should_unpack(src: &ArchiveSource, target_dir: &Path) -> Result<bool, ProtoArchiveError> {
    let url_file = target_dir.join(".archive-url");
    let mut unpack = true;

    // If the URLs have changed at some point, we need to remove
    // the current files, and download new ones
    if url_file.exists() {
        let previous_url = fs::read_file(&url_file)?;

        if src.url.trim() == previous_url.trim() {
            unpack = false;
        } else {
            fs::remove_dir_all(target_dir)?;
        }
    }

    fs::create_dir_all(target_dir)?;

    Ok(unpack)
}

pub async fn download(
    src: &ArchiveSource,
    temp_dir: &Path,
    options: DownloadOptions,
) -> Result<PathBuf, ProtoArchiveError> {
    let filename = extract_file_name_from_url(&src.url);
    let archive_file = temp_dir.join(&filename);

    net::download_from_url_with_options(&src.url, &archive_file, options).await?;

    Ok(archive_file)
}

pub async fn download_and_unpack(
    src: &ArchiveSource,
    target_dir: &Path,
    temp_dir: &Path,
    options: DownloadOptions,
) -> Result<(), ProtoArchiveError> {
    if should_unpack(src, target_dir)? {
        let archive_file = download(src, temp_dir, options).await?;

        unpack_source(src, target_dir, temp_dir, &archive_file).await?;
    }

    Ok(())
}

pub async fn unpack_source(
    src: &ArchiveSource,
    target_dir: &Path,
    temp_dir: &Path,
    archive_file: &Path,
) -> Result<(String, PathBuf), ProtoArchiveError> {
    let result = unpack(target_dir, temp_dir, archive_file, src.prefix.as_deref()).await;

    fs::write_file(target_dir.join(".archive-url"), &src.url)?;

    result
}

pub async fn unpack(
    target_dir: &Path,
    temp_dir: &Path,
    archive_file: &Path,
    prefix: Option<&str>,
) -> Result<(String, PathBuf), ProtoArchiveError> {
    match archive_file.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("pkg") => {
            unpack_pkg(target_dir, temp_dir, archive_file, prefix).await?;

            Ok(("pkg".into(), target_dir.to_path_buf()))
        }
        _ => {
            let mut archiver = Archiver::new(target_dir, archive_file);

            if let Some(prefix) = prefix {
                archiver.set_prefix(prefix);
            }

            Ok(archiver.unpack_from_ext()?)
        }
    }
}

async fn unpack_pkg(
    target_dir: &Path,
    temp_dir: &Path,
    archive_file: &Path,
    prefix: Option<&str>,
) -> Result<(), ProtoArchiveError> {
    let expanded_dir = temp_dir.join("pkg");
    let payload_dir = expanded_dir.join("Payload");

    fs::create_dir_all(temp_dir)?;

    // Remove expanded dir if it exists
    fs::remove_dir_all(&expanded_dir)?;

    let result = async {
        handle_exec(
            exec_command_piped(
                Command::new("pkgutil")
                    .arg("--expand-full")
                    .arg(archive_file)
                    .arg(&expanded_dir),
            )
            .await?,
        )?;

        if !payload_dir.exists() {
            return Err(ProtoArchiveError::MissingPkgPayload {
                path: expanded_dir.to_path_buf(),
            });
        }

        copy_extracted_contents(&payload_dir, target_dir, prefix)
    }
    .await;

    let _ = fs::remove_dir_all(&expanded_dir);

    result
}

fn copy_extracted_contents(
    source_dir: &Path,
    target_dir: &Path,
    prefix: Option<&str>,
) -> Result<(), ProtoArchiveError> {
    let source = match prefix {
        Some(prefix) => source_dir.join(prefix),
        None => source_dir.to_path_buf(),
    };

    if !source.exists() {
        return Err(ProtoArchiveError::MissingPkgContents {
            path: source_dir.to_path_buf(),
            prefix: prefix.unwrap_or("N/A").into(),
        });
    } else if source.is_file() {
        fs::copy_file(&source, target_dir.join(fs::file_name(&source)))?;
    } else {
        fs::copy_dir_all(&source, target_dir)?;
    }

    Ok(())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use starbase_sandbox::{Sandbox, create_empty_sandbox};
    use std::process::{Command as StdCommand, Stdio};

    fn has_macos_pkg_tools() -> bool {
        ["pkgbuild", "pkgutil"].into_iter().all(|bin| {
            StdCommand::new("which")
                .arg(bin)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        })
    }

    fn create_pkg(sandbox: &Sandbox, name: &str, files: &[(&str, &str, bool)]) -> PathBuf {
        let root = sandbox.path().join(format!("{name}-root"));

        for (relative_path, contents, executable) in files {
            let file = root.join(relative_path);

            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write_file(&file, contents).unwrap();

            #[cfg(unix)]
            if *executable {
                fs::update_perms(&file, Some(0o755)).unwrap();
            }
        }

        let package = sandbox.path().join(format!("{name}.pkg"));
        let output = StdCommand::new("pkgbuild")
            .arg("--root")
            .arg(&root)
            .arg("--identifier")
            .arg(format!("dev.proto.{name}"))
            .arg("--version")
            .arg("1.0.0")
            .arg(&package)
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "pkgbuild failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        package
    }

    #[tokio::test]
    async fn unpacks_pkg_payload_with_prefix() {
        if !has_macos_pkg_tools() {
            return;
        }

        let sandbox = create_empty_sandbox();
        let target_dir = sandbox.path().join("target");
        let temp_dir = sandbox.path().join("temp");
        let package = create_pkg(
            &sandbox,
            "prefixed",
            &[
                (
                    "Library/Developer/Toolchains/swift/bin/swift",
                    "#!/bin/sh\n",
                    true,
                ),
                (
                    "Library/Developer/Toolchains/swift/lib/libswift.dylib",
                    "library",
                    false,
                ),
            ],
        );

        fs::create_dir_all(&target_dir).unwrap();

        let (ext, unpacked_path) = unpack(
            &target_dir,
            &temp_dir,
            &package,
            Some("Library/Developer/Toolchains/swift"),
        )
        .await
        .unwrap();

        assert_eq!(ext, "pkg");
        assert_eq!(unpacked_path, target_dir);
        assert!(target_dir.join("bin/swift").is_file());
        assert!(target_dir.join("lib/libswift.dylib").is_file());
        assert!(!target_dir.join("Library").exists());
        assert!(!target_dir.join("Payload").exists());
        assert!(!target_dir.join("PackageInfo").exists());
        assert!(!temp_dir.join("pkg").exists());
    }

    #[tokio::test]
    async fn unpack_source_writes_archive_url_for_pkg() {
        if !has_macos_pkg_tools() {
            return;
        }

        let sandbox = create_empty_sandbox();
        let target_dir = sandbox.path().join("target");
        let temp_dir = sandbox.path().join("temp");
        let package = create_pkg(
            &sandbox,
            "source",
            &[("usr/local/bin/proto-tool", "#!/bin/sh\n", true)],
        );
        let source = ArchiveSource {
            url: "https://example.com/proto-tool.pkg".into(),
            prefix: Some("usr/local".into()),
        };

        fs::create_dir_all(&target_dir).unwrap();

        let (ext, unpacked_path) = unpack_source(&source, &target_dir, &temp_dir, &package)
            .await
            .unwrap();

        assert_eq!(ext, "pkg");
        assert_eq!(unpacked_path, target_dir);
        assert!(target_dir.join("bin/proto-tool").is_file());
        assert_eq!(
            fs::read_file(target_dir.join(".archive-url")).unwrap(),
            "https://example.com/proto-tool.pkg"
        );
        assert!(!temp_dir.join("pkg").exists());
    }
}
