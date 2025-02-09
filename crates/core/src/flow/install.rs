use super::build::*;
pub use super::build_error::ProtoBuildError;
pub use super::install_error::ProtoInstallError;
use crate::checksum::verify_checksum;
use crate::env::ProtoConsole;
use crate::env_error::ProtoEnvError;
use crate::helpers::{extract_filename_from_url, is_archive_file, is_offline};
use crate::tool::Tool;
use proto_pdk_api::*;
use proto_shim::*;
use starbase_archive::Archiver;
use starbase_styles::color;
use starbase_utils::net::DownloadOptions;
use starbase_utils::{fs, net};
use std::path::Path;
use system_env::System;
use tracing::{debug, instrument};

// Prebuilt: Download -> verify -> unpack
// Build: InstallDeps -> CheckRequirements -> ExecuteInstructions -> ...
#[derive(Clone, Debug)]
pub enum InstallPhase {
    Native,
    Download { url: String, file: String },
    Verify { url: String, file: String },
    Unpack { file: String },
    InstallDeps,
    CheckRequirements,
    ExecuteInstructions,
    CloneRepository { url: String },
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

        Err(ProtoInstallError::InvalidChecksum {
            checksum: checksum_file.to_path_buf(),
            download: download_file.to_path_buf(),
        }
        .into())
    }

    /// Build the tool from source using a set of requirements and instructions
    /// into the `~/.proto/tools/<version>` folder.
    #[instrument(skip(self, options))]
    pub async fn build_from_source(
        &self,
        install_dir: &Path,
        temp_dir: &Path,
        mut options: InstallOptions,
    ) -> miette::Result<()> {
        debug!(
            tool = self.id.as_str(),
            "Installing tool by building from source"
        );

        if !self.plugin.has_func("build_instructions").await {
            return Err(ProtoInstallError::UnsupportedBuildFromSource {
                tool: self.get_name().to_owned(),
            }
            .into());
        }

        let output: BuildInstructionsOutput = self
            .plugin
            .cache_func_with(
                "build_instructions",
                BuildInstructionsInput {
                    context: self.create_context(),
                },
            )
            .await?;

        let mut system = System::default();
        let config = self.proto.load_config()?;

        if let Some(pm) = config.settings.build.system_package_manager.get(&system.os) {
            if let Some(pm) = pm {
                system.manager = Some(*pm);

                debug!(
                    "Overwriting system package manager to {} for {}",
                    pm, system.os
                );
            } else {
                system.manager = None;

                debug!(
                    "Disabling system package manager because {} was disabled for {}",
                    color::property("settings.build.system-package-manager"),
                    system.os
                );
            }
        }

        let build_options = InstallBuildOptions {
            config: &config.settings.build,
            console: options.console.as_ref(),
            install_dir,
            http_client: self.proto.get_plugin_loader()?.get_client()?,
            on_phase_change: options.on_phase_change.take(),
            skip_prompts: options.skip_prompts,
            system,
            temp_dir,
            version: self.get_resolved_version(),
        };

        // The build process may require using itself to build itself,
        // so allow proto to use any available version instead of failing
        std::env::set_var(format!("{}_VERSION", self.get_env_var_prefix()), "*");

        // Step 0
        log_build_information(&output, &build_options)?;

        // Step 1
        if config.settings.build.install_system_packages {
            install_system_dependencies(&output, &build_options).await?;
        } else {
            debug!(
                "Not installing system dependencies because {} was disabled",
                color::property("settings.build.install-system-packages"),
            );
        }

        // Step 2
        check_requirements(&output, &build_options).await?;

        // Step 3
        download_sources(&output, &build_options).await?;

        // Step 4
        execute_instructions(&output, &build_options, &self.proto).await?;

        Ok(())
    }

    /// Download the tool (as an archive) from its distribution registry
    /// into the `~/.proto/tools/<version>` folder, and optionally verify checksums.
    #[instrument(skip(self, options))]
    pub async fn install_from_prebuilt(
        &self,
        install_dir: &Path,
        temp_dir: &Path,
        mut options: InstallOptions,
    ) -> miette::Result<()> {
        debug!(
            tool = self.id.as_str(),
            "Installing tool by downloading a pre-built archive"
        );

        if !self.plugin.has_func("download_prebuilt").await {
            return Err(ProtoInstallError::UnsupportedDownloadPrebuilt {
                tool: self.get_name().to_owned(),
            }
            .into());
        }

        let client = self.proto.get_plugin_loader()?.get_client()?;

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
            return Err(ProtoEnvError::RequiredInternetConnection.into());
        }

        let temp_dir = self.get_temp_dir();
        let install_dir = self.get_product_dir();
        let mut installed = false;

        // Lock the temporary directory instead of the install directory,
        // because the latter needs to be clean for "build from source",
        // and the `.lock` file breaks that contract
        let mut install_lock = fs::lock_directory(&temp_dir)?;

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
                return Err(ProtoInstallError::FailedInstall {
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
            let result = if matches!(options.strategy, InstallStrategy::BuildFromSource) {
                self.build_from_source(&install_dir, &temp_dir, options)
                    .await
            }
            // Install from a prebuilt archive
            else {
                self.install_from_prebuilt(&install_dir, &temp_dir, options)
                    .await
            };

            // Clean up if the install failed
            if let Err(error) = result {
                debug!(
                    tool = self.id.as_str(),
                    install_dir = ?install_dir,
                    "Failed to install tool, cleaning up",
                );

                install_lock.unlock()?;

                fs::remove_dir_all(&install_dir)?;
                fs::remove_dir_all(&temp_dir)?;

                return Err(error);
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
                return Err(ProtoInstallError::FailedUninstall {
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
