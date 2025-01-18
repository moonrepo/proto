use super::build::*;
use crate::checksum::verify_checksum;
use crate::error::ProtoError;
use crate::helpers::{extract_filename_from_url, is_archive_file, is_offline};
use crate::proto::ProtoConsole;
use crate::tool::Tool;
use proto_pdk_api::*;
use proto_shim::*;
use starbase_archive::Archiver;
use starbase_utils::net::DownloadOptions;
use starbase_utils::{fs, net};
use std::path::Path;
use tracing::{debug, instrument};

#[derive(Debug, Default)]
pub enum InstallStrategy {
    BuildFromSource,
    #[default]
    DownloadPrebuilt,
}

// Prebuilt: Download -> verify -> unpack
// Build: InstallDeps -> CheckRequirements
#[derive(Clone, Debug)]
pub enum InstallPhase {
    Native,
    Download { url: String, file: String },
    Verify { url: String, file: String },
    Unpack { file: String },
    InstallDeps,
    CheckRequirements,
}

pub use starbase_utils::net::OnChunkFn;
pub type OnPhaseFn = Box<dyn Fn(InstallPhase) + Send + Sync>;

#[derive(Default)]
pub struct InstallOptions {
    pub console: Option<ProtoConsole>,
    pub force: bool,
    pub on_download_chunk: Option<OnChunkFn>,
    pub on_phase_change: Option<OnPhaseFn>,
    pub skip_prompts: bool,
    pub strategy: InstallStrategy,
}

impl Tool {
    /// Return true if the tool has been installed. This is less accurate than `is_setup`,
    /// as it only checks for the existence of the inventory directory.
    pub fn is_installed(&self) -> bool {
        let dir = self.get_product_dir();

        self.version.as_ref().is_some_and(|v| {
            !v.is_latest() && self.inventory.manifest.installed_versions.contains(v)
        }) && dir.exists()
            && !fs::is_dir_locked(dir)
    }

    /// Verify the downloaded file using the checksum strategy for the tool.
    /// Common strategies are SHA256 and MD5.
    #[instrument(skip(self))]
    pub async fn verify_checksum(
        &self,
        checksum_file: &Path,
        download_file: &Path,
        checksum_public_key: Option<&str>,
    ) -> miette::Result<bool> {
        debug!(
            tool = self.id.as_str(),
            download_file = ?download_file,
            checksum_file = ?checksum_file,
            "Verifying checksum of downloaded file",
        );

        // Allow plugin to provide their own checksum verification method
        let verified = if self.plugin.has_func("verify_checksum").await {
            let output: VerifyChecksumOutput = self
                .plugin
                .call_func_with(
                    "verify_checksum",
                    VerifyChecksumInput {
                        checksum_file: self.to_virtual_path(checksum_file),
                        download_file: self.to_virtual_path(download_file),
                        context: self.create_context(),
                    },
                )
                .await?;

            output.verified

        // Otherwise attempt to verify it ourselves
        } else {
            verify_checksum(download_file, checksum_file, checksum_public_key)?
        };

        if verified {
            debug!(
                tool = self.id.as_str(),
                "Successfully verified, checksum matches"
            );

            return Ok(true);
        }

        Err(ProtoError::InvalidChecksum {
            checksum: checksum_file.to_path_buf(),
            download: download_file.to_path_buf(),
        }
        .into())
    }

    #[instrument(skip(self, options))]
    pub async fn build_from_source(
        &self,
        install_dir: &Path,
        mut options: InstallOptions,
    ) -> miette::Result<()> {
        debug!(
            tool = self.id.as_str(),
            "Installing tool by building from source"
        );

        if !self.plugin.has_func("build_instructions").await {
            return Err(ProtoError::UnsupportedBuildFromSource {
                tool: self.get_name().to_owned(),
            }
            .into());
        }

        // let temp_dir = self.get_temp_dir();

        let output: BuildInstructionsOutput = self
            .plugin
            .cache_func_with(
                "build_instructions",
                BuildInstructionsInput {
                    context: self.create_context(),
                },
            )
            .await?;

        let build_options = InstallBuildOptions {
            console: options.console.clone(),
            host_arch: HostArch::from_env(),
            host_os: HostOS::from_env(),
            on_phase_change: options.on_phase_change.take(),
            skip_prompts: options.skip_prompts,
        };

        // Step 1
        install_system_dependencies(&output.system_dependencies, &build_options).await?;

        // Step 2
        check_requirements(&output.requirements, &build_options).await?;

        std::process::exit(1);

        // match &options.source {
        //     // Should this do anything?
        //     SourceLocation::None => {
        //         return Ok(());
        //     }

        //     // Download from archive
        //     SourceLocation::Archive { url: archive_url } => {
        //         let download_file = temp_dir.join(extract_filename_from_url(archive_url)?);

        //         debug!(
        //             tool = self.id.as_str(),
        //             archive_url,
        //             download_file = ?download_file,
        //             install_dir = ?install_dir,
        //             "Attempting to download and unpack sources",
        //         );

        //         net::download_from_url_with_client(
        //             archive_url,
        //             &download_file,
        //             self.proto.get_plugin_loader()?.get_client()?,
        //         )
        //         .await?;

        //         Archiver::new(install_dir, &download_file).unpack_from_ext()?;
        //     }

        //     // Clone from Git repository
        //     SourceLocation::Git {
        //         url: repo_url,
        //         reference: ref_name,
        //         submodules,
        //     } => {
        //         debug!(
        //             tool = self.id.as_str(),
        //             repo_url,
        //             ref_name,
        //             install_dir = ?install_dir,
        //             "Attempting to clone a Git repository",
        //         );

        //         let run_git = |args: &[&str]| -> miette::Result<()> {
        //             let status = Command::new("git")
        //                 .args(args)
        //                 .current_dir(install_dir)
        //                 .spawn()
        //                 .into_diagnostic()?
        //                 .wait()
        //                 .into_diagnostic()?;

        //             if !status.success() {
        //                 return Err(ProtoError::BuildFailed {
        //                     tool: self.get_name().to_owned(),
        //                     url: repo_url.clone(),
        //                     status: format!("exit code {}", status),
        //                 }
        //                 .into());
        //             }

        //             Ok(())
        //         };

        //         // TODO, pull if already cloned

        //         fs::create_dir_all(install_dir)?;

        //         run_git(&[
        //             "clone",
        //             if *submodules {
        //                 "--recurse-submodules"
        //             } else {
        //                 ""
        //             },
        //             repo_url,
        //             ".",
        //         ])?;

        //         run_git(&["checkout", ref_name])?;
        //     }
        // };

        Ok(())
    }

    /// Download the tool (as an archive) from its distribution registry
    /// into the `~/.proto/tools/<version>` folder, and optionally verify checksums.
    #[instrument(skip(self, options))]
    pub async fn install_from_prebuilt(
        &self,
        install_dir: &Path,
        mut options: InstallOptions,
    ) -> miette::Result<()> {
        debug!(
            tool = self.id.as_str(),
            "Installing tool by downloading a pre-built archive"
        );

        let client = self.proto.get_plugin_loader()?.get_client()?;
        let temp_dir = self.get_temp_dir();

        let output: DownloadPrebuiltOutput = self
            .plugin
            .cache_func_with(
                "download_prebuilt",
                DownloadPrebuiltInput {
                    context: self.create_context(),
                    install_dir: self.to_virtual_path(install_dir),
                },
            )
            .await?;

        // Download the prebuilt
        let download_url = output.download_url;
        let download_filename = match output.download_name {
            Some(name) => name,
            None => extract_filename_from_url(&download_url)?,
        };
        let download_file = temp_dir.join(&download_filename);

        options.on_phase_change.as_ref().inspect(|func| {
            func(InstallPhase::Download {
                url: download_url.clone(),
                file: download_filename.clone(),
            });
        });

        debug!(tool = self.id.as_str(), "Downloading tool archive");

        net::download_from_url_with_options(
            &download_url,
            &download_file,
            DownloadOptions {
                downloader: Some(Box::new(client.create_downloader())),
                on_chunk: options.on_download_chunk.take(),
            },
        )
        .await?;

        // Verify the checksum if applicable
        if let Some(checksum_url) = output.checksum_url {
            let checksum_filename = match output.checksum_name {
                Some(name) => name,
                None => extract_filename_from_url(&checksum_url)?,
            };
            let checksum_file = temp_dir.join(&checksum_filename);

            options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::Verify {
                    url: checksum_url.clone(),
                    file: checksum_filename.clone(),
                });
            });

            debug!(tool = self.id.as_str(), "Downloading tool checksum");

            net::download_from_url_with_options(
                &checksum_url,
                &checksum_file,
                DownloadOptions {
                    downloader: Some(Box::new(client.create_downloader())),
                    on_chunk: None,
                },
            )
            .await?;

            self.verify_checksum(
                &checksum_file,
                &download_file,
                output.checksum_public_key.as_deref(),
            )
            .await?;
        }

        // Attempt to unpack the archive
        debug!(
            tool = self.id.as_str(),
            download_file = ?download_file,
            install_dir = ?install_dir,
            "Attempting to unpack archive",
        );

        if self.plugin.has_func("unpack_archive").await {
            options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::Unpack {
                    file: download_filename.clone(),
                });
            });

            self.plugin
                .call_func_without_output(
                    "unpack_archive",
                    UnpackArchiveInput {
                        input_file: self.to_virtual_path(&download_file),
                        output_dir: self.to_virtual_path(install_dir),
                        context: self.create_context(),
                    },
                )
                .await?;
        }
        // Is an archive, unpack it
        else if is_archive_file(&download_file) {
            options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::Unpack {
                    file: download_filename.clone(),
                });
            });

            let mut archiver = Archiver::new(install_dir, &download_file);

            if let Some(prefix) = &output.archive_prefix {
                archiver.set_prefix(prefix);
            }

            let (ext, unpacked_path) = archiver.unpack_from_ext()?;

            // If the archive was `.gz` without tar or other formats,
            // it's a single file, so assume a binary and update perms
            if ext == "gz" && unpacked_path.is_file() {
                fs::update_perms(unpacked_path, None)?;
            }
        }
        // Not an archive, assume a binary and copy
        else {
            let install_path = install_dir.join(get_exe_file_name(&self.id));

            fs::rename(&download_file, &install_path)?;
            fs::update_perms(install_path, None)?;
        }

        Ok(())
    }

    /// Install a tool into proto, either by downloading and unpacking
    /// a pre-built archive, or by using a native installation method.
    #[instrument(skip(self, options))]
    pub async fn install(&mut self, options: InstallOptions) -> miette::Result<bool> {
        if self.is_installed() && !options.force {
            debug!(
                tool = self.id.as_str(),
                "Tool already installed, continuing"
            );

            return Ok(false);
        }

        if is_offline() {
            return Err(ProtoError::InternetConnectionRequired.into());
        }

        let install_dir = self.get_product_dir();
        let mut installed = false;

        // Lock the install directory. If the inventory has been overridden,
        // lock the internal proto tool directory instead.
        let _install_lock =
            fs::lock_directory(if self.metadata.inventory.override_dir.is_some() {
                self.proto
                    .store
                    .inventory_dir
                    .join(self.id.as_str())
                    .join(self.get_resolved_version().to_string())
            } else {
                install_dir.clone()
            })?;

        // If this function is defined, it acts like an escape hatch and
        // takes precedence over all other install strategies
        if self.plugin.has_func("native_install").await {
            debug!(tool = self.id.as_str(), "Installing tool natively");

            options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::Native);
            });

            let output: NativeInstallOutput = self
                .plugin
                .call_func_with(
                    "native_install",
                    NativeInstallInput {
                        context: self.create_context(),
                        install_dir: self.to_virtual_path(&install_dir),
                    },
                )
                .await?;

            if !output.installed && !output.skip_install {
                return Err(ProtoError::InstallFailed {
                    tool: self.get_name().to_owned(),
                    error: output.error.unwrap_or_default(),
                }
                .into());

            // If native install fails, attempt other installers
            } else {
                installed = output.installed;
            }
        }

        if !installed {
            // Build the tool from source
            if matches!(options.strategy, InstallStrategy::BuildFromSource) {
                self.build_from_source(&install_dir, options).await?;
            }
            // Install from a prebuilt archive
            else {
                self.install_from_prebuilt(&install_dir, options).await?;
            }
        }

        debug!(
            tool = self.id.as_str(),
            install_dir = ?install_dir,
            "Successfully installed tool",
        );

        Ok(true)
    }

    /// Uninstall the tool by deleting the current install directory.
    #[instrument(skip_all)]
    pub async fn uninstall(&self) -> miette::Result<bool> {
        let install_dir = self.get_product_dir();

        if !install_dir.exists() {
            debug!(
                tool = self.id.as_str(),
                "Tool has not been installed, aborting"
            );

            return Ok(false);
        }

        if self.plugin.has_func("native_uninstall").await {
            debug!(tool = self.id.as_str(), "Uninstalling tool natively");

            let output: NativeUninstallOutput = self
                .plugin
                .call_func_with(
                    "native_uninstall",
                    NativeUninstallInput {
                        context: self.create_context(),
                    },
                )
                .await?;

            if !output.uninstalled && !output.skip_uninstall {
                return Err(ProtoError::UninstallFailed {
                    tool: self.get_name().to_owned(),
                    error: output.error.unwrap_or_default(),
                }
                .into());
            }
        }

        debug!(
            tool = self.id.as_str(),
            install_dir = ?install_dir,
            "Deleting install directory"
        );

        fs::remove_dir_all(install_dir)?;

        debug!(tool = self.id.as_str(), "Successfully uninstalled tool");

        Ok(true)
    }
}
