use super::build::*;
pub use super::build_error::ProtoBuildError;
pub use super::install_error::ProtoInstallError;
use crate::checksum::*;
use crate::env::ProtoConsole;
use crate::env_error::ProtoEnvError;
use crate::helpers::{extract_filename_from_url, is_archive_file, is_offline};
use crate::lockfile::*;
use crate::tool::Tool;
use crate::utils::archive;
use proto_pdk_api::*;
use proto_shim::*;
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
    pub skip_ui: bool,
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
    ) -> miette::Result<Checksum> {
        debug!(
            tool = self.id.as_str(),
            download_file = ?download_file,
            checksum_file = ?checksum_file,
            "Verifying checksum of downloaded file",
        );

        let checksum = generate_checksum(download_file, checksum_file, checksum_public_key)?;
        let verified;

        // Allow plugin to provide their own checksum verification method
        if self.plugin.has_func("verify_checksum").await {
            let output: VerifyChecksumOutput = self
                .plugin
                .call_func_with(
                    "verify_checksum",
                    VerifyChecksumInput {
                        checksum_file: self.to_virtual_path(checksum_file),
                        download_file: self.to_virtual_path(download_file),
                        download_checksum: Some(checksum.clone()),
                        context: self.create_context(),
                    },
                )
                .await?;

            verified = output.verified;
        }
        // Otherwise attempt to verify it ourselves
        else {
            verified = verify_checksum(download_file, checksum_file, &checksum)?;
        }

        if verified {
            debug!(
                tool = self.id.as_str(),
                "Successfully verified, checksum matches"
            );

            return Ok(checksum);
        }

        Err(ProtoInstallError::InvalidChecksum {
            checksum: checksum_file.to_path_buf(),
            download: download_file.to_path_buf(),
        }
        .into())
    }

    /// Verify the installation is legitimate by comparing it to a lockfile record.
    #[instrument(skip(self))]
    pub fn verify_lockfile(&self, record: &LockfileRecord) -> miette::Result<()> {
        let Some(version) = &self.version else {
            return Ok(());
        };

        // No lockfile record yet
        let Some(lockfile) = self.inventory.get_locked_record(version) else {
            return Ok(());
        };

        // If we have different backends, then the installation strategy
        // and content/files which are hashed may differ, so avoid verify
        if record.backend != lockfile.backend {
            return Ok(());
        }

        let make_error = |actual: String, expected: String| match &record.source {
            Some(source) => ProtoInstallError::MismatchedChecksumWithSource {
                checksum: actual,
                lockfile_checksum: expected,
                source_url: source.to_owned(),
            },
            None => ProtoInstallError::MismatchedChecksum {
                checksum: actual,
                lockfile_checksum: expected,
            },
        };

        match (&record.checksum, &lockfile.checksum) {
            (Some(rc), Some(lc)) => {
                debug!(
                    tool = self.id.as_str(),
                    checksum = rc.to_string(),
                    "Verifying checksum against lockfile",
                );

                if rc != lc {
                    return Err(make_error(rc.to_string(), lc.to_string()).into());
                }
            }
            // Only the lockfile has a checksum, so compare the sources.
            // If the sources are the same, something wrong is happening,
            // but if they are different, then it may be a different install
            // strategy, so let it happen
            (None, Some(lc)) => {
                if record.source == lockfile.source {
                    return Err(make_error("(missing)".into(), lc.to_string()).into());
                }
            }
            _ => {}
        };

        Ok(())
    }

    /// Build the tool from source using a set of requirements and instructions
    /// into the `~/.proto/tools/<version>` folder.
    #[instrument(skip(self, options))]
    pub async fn build_from_source(
        &self,
        install_dir: &Path,
        temp_dir: &Path,
        mut options: InstallOptions,
    ) -> miette::Result<LockfileRecord> {
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
                    install_dir: self.to_virtual_path(install_dir),
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

        let mut builder = Builder::new(BuilderOptions {
            config: &config.settings.build,
            console: options
                .console
                .as_ref()
                .expect("Console required for builder!"),
            install_dir,
            http_client: self.proto.get_plugin_loader()?.get_client()?,
            on_phase_change: options.on_phase_change.take(),
            skip_prompts: options.skip_prompts,
            skip_ui: options.skip_ui,
            system,
            temp_dir,
            version: self.get_resolved_version(),
        });

        // If any step in the build process fails, we should write
        // a log file so that the user can debug it, otherwise the
        // piped commands are hidden from the user
        let handle_error = |result: miette::Result<()>, instance: &Builder| {
            if
            // Always write
            instance.options.config.write_log_file ||
                // Only write if an error and no direct UI
                result.is_err() && instance.options.skip_ui
            {
                instance.write_log_file(
                    self.proto
                        .working_dir
                        .join(format!("proto-{}-build.log", self.id)),
                )?;
            }

            result
        };

        // The build process may require using itself to build itself,
        // so allow proto to use any available version instead of failing
        unsafe { std::env::set_var(format!("{}_VERSION", self.get_env_var_prefix()), "*") };

        let mut record = LockfileRecord::new(self.backend);

        // Step 0
        handle_error(log_build_information(&mut builder, &output), &builder)?;

        // Step 1
        if config.settings.build.install_system_packages {
            handle_error(
                install_system_dependencies(&mut builder, &output).await,
                &builder,
            )?;
        } else {
            debug!(
                "Not installing system dependencies because {} was disabled",
                color::property("settings.build.install-system-packages"),
            );
        }

        // Step 2
        handle_error(check_requirements(&mut builder, &output).await, &builder)?;

        // Step 3
        handle_error(
            download_sources(&mut builder, &output, &mut record).await,
            &builder,
        )?;

        // Step 4
        handle_error(
            execute_instructions(&mut builder, &output, &self.proto).await,
            &builder,
        )?;

        Ok(record)
    }

    /// Download the tool (as an archive) from its distribution registry
    /// into the `~/.proto/tools/<version>` folder, and optionally verify checksums.
    #[instrument(skip(self, options))]
    pub async fn install_from_prebuilt(
        &self,
        install_dir: &Path,
        temp_dir: &Path,
        mut options: InstallOptions,
    ) -> miette::Result<LockfileRecord> {
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

        let mut record = LockfileRecord::new(self.backend);

        // Download the prebuilt
        let download_url = output.download_url;
        let download_filename = match output.download_name {
            Some(name) => name,
            None => extract_filename_from_url(&download_url)?,
        };
        let download_file = temp_dir.join(&download_filename);

        record.source = Some(download_url.clone());
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

        // Verify against a URL that contains the checksum
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

            record.checksum = Some(
                self.verify_checksum(
                    &checksum_file,
                    &download_file,
                    output.checksum_public_key.as_deref(),
                )
                .await?,
            );
        }
        // Verify against an explicitly provided checksum
        else if let Some(checksum) = output.checksum {
            let checksum_file = temp_dir.join("CHECKSUM");

            fs::write_file(&checksum_file, checksum.to_string())?;

            debug!(
                tool = self.id.as_str(),
                checksum = checksum.to_string(),
                "Using provided checksum"
            );

            record.checksum = Some(
                self.verify_checksum(
                    &checksum_file,
                    &download_file,
                    output.checksum_public_key.as_deref(),
                )
                .await?,
            );
        }
        // No available checksum, so generate one ourselves for the lockfile
        else {
            record.checksum = Some(Checksum::sha256(hash_file_contents_sha256(&download_file)?));
        }

        // Verify against lockfile
        self.verify_lockfile(&record)?;

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

            let (ext, unpacked_path) = archive::unpack_raw(
                install_dir,
                &download_file,
                output.archive_prefix.as_deref(),
            )?;

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

        Ok(record)
    }

    /// Install a tool into proto, either by downloading and unpacking
    /// a pre-built archive, or by using a native installation method.
    #[instrument(skip(self, options))]
    pub async fn install(
        &mut self,
        options: InstallOptions,
    ) -> miette::Result<Option<LockfileRecord>> {
        if self.is_installed() && !options.force {
            debug!(
                tool = self.id.as_str(),
                "Tool already installed, continuing"
            );

            return Ok(None);
        }

        if is_offline() {
            return Err(ProtoEnvError::RequiredInternetConnection.into());
        }

        let temp_dir = self.get_temp_dir();
        let install_dir = self.get_product_dir();

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

            fs::create_dir_all(&install_dir)?;

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

            if output.installed {
                let mut record = LockfileRecord::new(self.backend);
                record.checksum = output.checksum;

                return Ok(Some(record));
            }

            if !output.skip_install {
                return Err(ProtoInstallError::FailedInstall {
                    tool: self.get_name().to_owned(),
                    error: output.error.unwrap_or_default(),
                }
                .into());
            }
        }

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

        match result {
            Ok(record) => {
                debug!(
                    tool = self.id.as_str(),
                    install_dir = ?install_dir,
                    "Successfully installed tool",
                );

                Ok(Some(record))
            }

            // Clean up if the install failed
            Err(error) => {
                debug!(
                    tool = self.id.as_str(),
                    install_dir = ?install_dir,
                    "Failed to install tool, cleaning up",
                );

                install_lock.unlock()?;

                fs::remove_dir_all(&install_dir)?;
                fs::remove_dir_all(&temp_dir)?;

                Err(error)
            }
        }
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
                        uninstall_dir: self.to_virtual_path(&install_dir),
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
