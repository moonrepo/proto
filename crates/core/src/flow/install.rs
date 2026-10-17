use super::build::*;
pub use super::build_error::ProtoBuildError;
pub use super::install_error::ProtoInstallError;
use crate::checksum::*;
use crate::flow::lock::{LockOutcome, LockSkipReason, Locker, ProtoLockError};
use crate::helpers::{is_archive_file, is_offline};
use crate::lockfile::*;
use crate::reporter::ProtoConsole;
use crate::tool::Tool;
use crate::tool_spec::ToolSpec;
use crate::utils::log::LogWriter;
use crate::utils::{archive, process};
use proto_pdk_api::*;
use starbase_archive::get_compression_extensions;
use starbase_shell::{ShellType, join_exe_args};
use starbase_styles::color;
use starbase_utils::net::DownloadOptions;
use starbase_utils::{fs, hash, net, path};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use system_env::System;
use tokio::process::Command;
use tracing::{debug, instrument, warn};
use warpgate::extract_file_name_from_url;

pub use starbase_utils::net::OnChunkFn;
pub type OnPhaseFn = Arc<dyn Fn(InstallPhase) + Send + Sync>;

// Prebuilt: Download -> Verify -> Unpack
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

#[derive(Clone, Default)]
pub struct InstallOptions {
    pub console: Option<ProtoConsole>,
    pub force: bool,
    pub log_writer: Option<LogWriter>,
    pub on_download_chunk: Option<OnChunkFn>,
    pub on_phase_change: Option<OnPhaseFn>,
    pub skip_prompts: bool,
    pub skip_ui: bool,
    pub strategy: InstallStrategy,
}

/// A pre-built that has been downloaded and verified, but not unpacked.
pub struct PreparedPrebuilt {
    /// Absolute path to the downloaded file, within the temp directory.
    pub download_file: PathBuf,

    /// File name of the downloaded file.
    pub download_name: String,

    /// Output returned by the plugin's `download_prebuilt` function.
    pub output: DownloadPrebuiltOutput,

    /// Lock record with the checksum and source of the download.
    pub record: LockRecord,
}

/// Installs a tool into proto's store.
pub struct Installer<'tool> {
    tool: &'tool Tool,
    spec: &'tool ToolSpec,

    pub product_dir: PathBuf,
    pub temp_dir: PathBuf,
}

impl<'tool> Installer<'tool> {
    pub fn new(tool: &'tool Tool, spec: &'tool ToolSpec) -> Self {
        Self {
            product_dir: tool.get_product_dir(spec),
            temp_dir: tool.get_version_temp_dir(spec),
            tool,
            spec,
        }
    }

    /// Install a tool into proto, either by downloading and unpacking
    /// a pre-built archive, or by using a native installation method.
    ///
    /// This method does not guard against concurrent installs, and does not
    /// clean up partial state when erroring. Cross-process mutual exclusion,
    /// completion bookkeeping, and cleanup are provided by
    /// [`Manager::install`](crate::flow::manage::Manager), which wraps this
    /// call in a lock on [`Installer::temp_dir`].
    #[instrument(skip(self, options))]
    pub async fn install(
        &self,
        options: InstallOptions,
    ) -> Result<Option<LockRecord>, ProtoInstallError> {
        if self.tool.is_installed_while_locked(self.spec) && !options.force {
            debug!(
                tool = self.tool.context.as_str(),
                "Tool already installed, continuing"
            );

            return Ok(None);
        }

        if is_offline() {
            return Err(ProtoInstallError::RequiredInternetConnection);
        }

        // Downloads (and the lock file managed by the caller) live here
        fs::create_dir_all(&self.temp_dir)?;

        // If this function is defined, it acts like an escape hatch and
        // takes precedence over all other install strategies
        if self
            .tool
            .plugin
            .has_func(PluginFunction::NativeInstall)
            .await
        {
            debug!(
                tool = self.tool.context.as_str(),
                "Installing tool natively"
            );

            options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::Native);
            });

            fs::create_dir_all(&self.product_dir)?;

            let output: NativeInstallOutput =
                self.tool.proto.create_metric().record_tool_install_step(
                    &self.tool.context,
                    "native_install",
                    self.tool
                        .plugin
                        .call_func_with(
                            PluginFunction::NativeInstall,
                            NativeInstallInput {
                                context: self.tool.create_plugin_context(self.spec),
                                install_dir: self.tool.to_virtual_path(&self.product_dir),
                                force: options.force,
                            },
                        )
                        .await,
                )?;

            if output.installed {
                let mut record = self.tool.create_locked_record();
                record.checksum = output.checksum;

                // Verify against lockfile
                Locker::new(self.tool).verify_locked_record(self.spec, &record)?;

                return Ok(Some(record));
            }

            if !output.skip_install {
                return Err(ProtoInstallError::FailedInstall {
                    tool: self.tool.get_name().to_owned(),
                    error: output.error.unwrap_or_default(),
                });
            }
        }

        // Build the tool from source
        let result = if matches!(options.strategy, InstallStrategy::BuildFromSource) {
            self.build_from_source(options).await
        }
        // Install from a prebuilt archive
        else {
            self.install_from_prebuilt(options).await
        };

        // Ensure that we installed something
        let record = match result {
            Ok(record) => {
                if !self.product_dir.exists() || fs::read_dir(&self.product_dir)?.is_empty() {
                    return Err(ProtoInstallError::FailedInstallNoFiles {
                        tool: self.tool.get_name().to_owned(),
                        dir: self.product_dir.clone(),
                    });
                }

                record
            }
            Err(error) => {
                return Err(error);
            }
        };

        // Verify against lockfile
        Locker::new(self.tool).verify_locked_record(self.spec, &record)?;

        debug!(
            tool = self.tool.context.as_str(),
            install_dir = ?self.product_dir,
            "Successfully installed tool",
        );

        Ok(Some(record))
    }

    /// Lock the tool without installing it, by downloading and verifying the
    /// pre-built for the tool's platform, and writing its record to the lockfile.
    /// Tools that can only be verified by installing them (native installs and
    /// builds from source) are skipped.
    ///
    /// When a record with a checksum already exists for the version and
    /// platform, nothing is downloaded, unless `force` is enabled, in which
    /// case the download must match the record's checksum.
    ///
    /// This *requires* the spec to have been resolved before hand.
    #[instrument(skip(self, options))]
    pub async fn lock(&self, options: InstallOptions) -> Result<LockOutcome, ProtoInstallError> {
        debug_assert!(self.spec.is_resolved(), "Spec must be resolved to lock!");

        let locker = Locker::new(self.tool);
        let plugin = &self.tool.plugin;

        let skip_reason = if self.tool.metadata.lock_options.no_record {
            Some(LockSkipReason::NoRecord)
        } else if !locker.has_lockfile()? {
            Some(LockSkipReason::NoLockfile)
        } else if plugin.has_func(PluginFunction::NativeInstall).await {
            Some(LockSkipReason::NativeInstall)
        } else if matches!(options.strategy, InstallStrategy::BuildFromSource) {
            Some(LockSkipReason::BuildFromSource)
        } else if !plugin.has_func(PluginFunction::DownloadPrebuilt).await {
            Some(LockSkipReason::NoPrebuilt)
        } else {
            None
        };

        if let Some(reason) = skip_reason {
            debug!(
                tool = self.tool.context.as_str(),
                reason = ?reason,
                "Unable to lock tool without installing it, skipping",
            );

            return Ok(LockOutcome::Skipped(reason));
        }

        let version = self.spec.get_resolved_version();
        let platform = self.tool.get_platform();

        let mut record = self.tool.create_locked_record();
        record.version = Some(version.clone());
        record.spec = Some(self.spec.req.clone());
        record.metadata = self.tool.metadata.lock_options.metadata.clone();

        // Only a record for the same platform can verify the download, as other
        // platforms download different artifacts. The record that the spec was
        // resolved with can't be used, as it's for the host machine
        let existing = locker
            .find_platform_record(&record)?
            .filter(|existing| existing.version == record.version && existing.checksum.is_some());

        if let Some(existing) = &existing
            && !options.force
        {
            debug!(
                tool = self.tool.context.as_str(),
                platform = platform.to_string(),
                "Tool has already been locked for this version and platform, skipping",
            );

            return Ok(LockOutcome::AlreadyLocked(existing.to_owned()));
        }

        if is_offline() {
            return Err(ProtoInstallError::RequiredInternetConnection);
        }

        debug!(
            tool = self.tool.context.as_str(),
            platform = platform.to_string(),
            "Locking tool by downloading a pre-built archive",
        );

        // The version's temp directory is owned by installs, which remove it
        // when they finish, so stage downloads in a directory unique to the
        // platform and this process instead
        let lock_dir = self.tool.get_temp_dir().join("lock");
        let temp_dir = lock_dir.join(hash::base64::from_bytes(format!(
            "{platform}-{version}-{}",
            std::process::id()
        )));

        let result = self.prepare_prebuilt(&temp_dir, &options).await;

        let _ = fs::remove_dir_all(&temp_dir);
        let _ = std::fs::remove_dir(&lock_dir);

        let prepared = result?;

        record.checksum = prepared.record.checksum;
        record.source = prepared.record.source;

        if let Some(existing) = existing
            && let (Some(checksum), Some(locked_checksum)) = (&record.checksum, &existing.checksum)
            && checksum != locked_checksum
        {
            return Err(match &record.source {
                Some(source) => ProtoLockError::MismatchedChecksumWithSource {
                    checksum: checksum.to_string(),
                    lockfile_checksum: locked_checksum.to_string(),
                    source_url: source.to_owned(),
                },
                None => ProtoLockError::MismatchedChecksum {
                    checksum: checksum.to_string(),
                    lockfile_checksum: locked_checksum.to_string(),
                },
            }
            .into());
        }

        locker.replace_record_in_lockfile(&record)?;

        Ok(LockOutcome::Locked(record))
    }

    /// Build the tool from source using a set of requirements and instructions
    /// into the `~/.proto/tools/<version>` folder.
    #[instrument(skip(self, options))]
    async fn build_from_source(
        &self,
        options: InstallOptions,
    ) -> Result<LockRecord, ProtoInstallError> {
        debug!(
            tool = self.tool.context.as_str(),
            "Installing tool by building from source"
        );

        if !self
            .tool
            .plugin
            .has_func(PluginFunction::BuildInstructions)
            .await
        {
            return Err(ProtoInstallError::UnsupportedBuildFromSource {
                tool: self.tool.get_name().to_owned(),
            });
        }

        let proto = &self.tool.proto;

        let output: BuildInstructionsOutput = proto.create_metric().record_tool_install_step(
            &self.tool.context,
            "prepare_build_instructions",
            self.tool
                .plugin
                .cache_func_with(
                    PluginFunction::BuildInstructions,
                    BuildInstructionsInput {
                        context: self.tool.create_plugin_context(self.spec),
                        install_dir: self.tool.to_virtual_path(&self.product_dir),
                    },
                )
                .await,
        )?;

        let mut system = System::default();
        let config = proto.load_config()?;

        if let Some(pm) = config.settings.build.system_package_manager.get(&system.os) {
            if let Some(pm) = pm {
                system.manager = Some(*pm);

                debug!(
                    tool = self.tool.context.as_str(),
                    "Overwriting system package manager to {} for {}", pm, system.os
                );
            } else {
                system.manager = None;

                debug!(
                    tool = self.tool.context.as_str(),
                    "Disabling system package manager because {} was disabled for {}",
                    color::property("settings.build.system-package-manager"),
                    system.os
                );
            }
        }

        let record = Builder::new(
            self.tool,
            BuilderOptions {
                config,
                console: options.console.as_ref(),
                install_dir: &self.product_dir,
                http_client: proto.get_plugin_loader()?.get_http_client()?,
                log_writer: options
                    .log_writer
                    .as_ref()
                    .expect("Logger required for builder!"),
                on_phase_change: options.on_phase_change.clone(),
                skip_prompts: options.skip_prompts,
                skip_ui: options.skip_ui,
                system,
                temp_dir: &self.temp_dir,
                version: self.spec.get_resolved_version(),
            },
        )
        .build(output)
        .await?;

        Ok(record)
    }

    /// Download the tool (as an archive) from its distribution registry into
    /// the provided temp directory, and verify it against a checksum, or generate
    /// one when none is available. The download is not unpacked, so the returned
    /// lock record can be created without installing the tool.
    #[instrument(skip(self, options))]
    pub async fn prepare_prebuilt(
        &self,
        temp_dir: &Path,
        options: &InstallOptions,
    ) -> Result<PreparedPrebuilt, ProtoInstallError> {
        if !self
            .tool
            .plugin
            .has_func(PluginFunction::DownloadPrebuilt)
            .await
        {
            return Err(ProtoInstallError::UnsupportedDownloadPrebuilt {
                tool: self.tool.get_name().to_owned(),
            });
        }

        let proto = &self.tool.proto;
        let client = proto.get_plugin_loader()?.get_http_client()?;
        let config = proto.load_config()?;

        let output: DownloadPrebuiltOutput = proto.create_metric().record_tool_install_step(
            &self.tool.context,
            "prepare_download_prebuilt",
            self.tool
                .plugin
                .cache_func_with(
                    PluginFunction::DownloadPrebuilt,
                    DownloadPrebuiltInput {
                        context: self.tool.create_plugin_context(self.spec),
                        install_dir: self.tool.to_virtual_path(&self.product_dir),
                    },
                )
                .await,
        )?;

        let mut record = self.tool.create_locked_record();

        // Download the prebuilt
        let download_url = config.rewrite_url(&output.download_url);
        let download_name = output
            .download_name
            .clone()
            .unwrap_or_else(|| extract_file_name_from_url(&download_url));
        let download_file = temp_dir.join(&download_name);

        // Already created when installing, but not when locking
        fs::create_dir_all(temp_dir)?;

        record.source = Some(download_url.clone());
        options.on_phase_change.as_ref().inspect(|func| {
            func(InstallPhase::Download {
                url: download_url.clone(),
                file: download_name.clone(),
            });
        });

        debug!(
            tool = self.tool.context.as_str(),
            "Downloading tool archive"
        );

        proto.create_metric().record_tool_install_step(
            &self.tool.context,
            "download_archive",
            net::download_from_url_with_options(
                &download_url,
                &download_file,
                DownloadOptions {
                    downloader: Some(Box::new(
                        client.create_downloader_with_headers(output.http_headers.clone()),
                    )),
                    on_chunk: options.on_download_chunk.clone(),
                    ..Default::default()
                },
            )
            .await,
        )?;

        // Verify against a URL that contains the checksum
        if let Some(checksum_url) = &output.checksum_url {
            let checksum_url = config.rewrite_url(checksum_url);
            let checksum_filename = output
                .checksum_name
                .clone()
                .unwrap_or_else(|| extract_file_name_from_url(&checksum_url));
            let checksum_file = temp_dir.join(&checksum_filename);

            options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::Verify {
                    url: checksum_url.clone(),
                    file: checksum_filename.clone(),
                });
            });

            debug!(
                tool = self.tool.context.as_str(),
                "Downloading tool checksum"
            );

            proto.create_metric().record_tool_install_step(
                &self.tool.context,
                "download_checksum",
                net::download_from_url_with_options(
                    &checksum_url,
                    &checksum_file,
                    DownloadOptions::new(
                        client.create_downloader_with_headers(output.http_headers.clone()),
                    ),
                )
                .await,
            )?;

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
        else if let Some(checksum) = &output.checksum {
            let checksum_file =
                temp_dir.join(format!("CHECKSUM.{:?}", checksum.algo).to_lowercase());

            fs::write_file(&checksum_file, checksum.hash.as_deref().unwrap_or_default())?;

            debug!(
                tool = self.tool.context.as_str(),
                checksum = checksum.to_string(),
                "Using provided checksum"
            );

            record.checksum = Some(
                self.verify_checksum(
                    &checksum_file,
                    &download_file,
                    output
                        .checksum_public_key
                        .as_deref()
                        .or(checksum.key.as_deref()),
                )
                .await?,
            );
        }
        // No available checksum, so generate one ourselves for the lockfile
        else {
            record.checksum = Some(Checksum::sha256(
                proto.create_metric().record_tool_install_step(
                    &self.tool.context,
                    "generate_checksum",
                    hash_file_contents_sha256(&download_file),
                )?,
            ));
        }

        Ok(PreparedPrebuilt {
            download_file,
            download_name,
            output,
            record,
        })
    }

    /// Download and verify the tool (as an archive) with [`Installer::prepare_prebuilt`],
    /// then unpack it into the `~/.proto/tools/<version>` folder.
    #[instrument(skip(self, options))]
    async fn install_from_prebuilt(
        &self,
        options: InstallOptions,
    ) -> Result<LockRecord, ProtoInstallError> {
        debug!(
            tool = self.tool.context.as_str(),
            "Installing tool by downloading a pre-built archive"
        );

        let PreparedPrebuilt {
            download_file,
            download_name,
            output,
            record,
        } = self.prepare_prebuilt(&self.temp_dir, &options).await?;

        let proto = &self.tool.proto;

        // Attempt to unpack the archive
        debug!(
            tool = self.tool.context.as_str(),
            download_file = ?download_file,
            install_dir = ?self.product_dir,
            "Attempting to unpack archive",
        );

        if self
            .tool
            .plugin
            .has_func(PluginFunction::UnpackArchive)
            .await
        {
            options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::Unpack {
                    file: download_name.clone(),
                });
            });

            proto.create_metric().record_tool_install_step(
                &self.tool.context,
                "unpack_archive",
                self.tool
                    .plugin
                    .call_func_without_output(
                        PluginFunction::UnpackArchive,
                        UnpackArchiveInput {
                            input_file: self.tool.to_virtual_path(&download_file),
                            output_dir: self.tool.to_virtual_path(&self.product_dir),
                            context: self.tool.create_plugin_context(self.spec),
                        },
                    )
                    .await,
            )?;
        }
        // Is an archive, unpack it
        else if is_archive_file(&download_file) {
            options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::Unpack {
                    file: download_name.clone(),
                });
            });

            let (ext, unpacked_path) = proto.create_metric().record_tool_install_step(
                &self.tool.context,
                "unpack_archive",
                archive::unpack(
                    &self.product_dir,
                    &self.temp_dir,
                    &download_file,
                    output.archive_prefix.as_deref(),
                )
                .await,
            )?;

            // If the archive was compressed without tar or other formats,
            // it's a single file, so assume a file and update perms
            if get_compression_extensions().contains(&ext) && unpacked_path.is_file() {
                fs::update_perms(unpacked_path, None)?;
            }
        }
        // Not an archive, assume a file and copy
        else {
            let install_path = self.product_dir.join(path::exe_name(path::encode_component(
                self.tool.get_file_name(),
            )));

            proto.create_metric().record_tool_install_step(
                &self.tool.context,
                "copy_executable",
                fs::rename(&download_file, &install_path)
                    .and_then(|_| fs::update_perms(install_path, None)),
            )?;
        }

        // Execute post install script
        if let Some(rel_script) = output.post_script {
            let abs_script = self.product_dir.join(rel_script);

            if !abs_script.exists() {
                warn!(
                    tool = self.tool.context.as_str(),
                    script = ?abs_script,
                    "Post-install script does not exist",
                );
            } else if !fs::is_executable(&abs_script) {
                warn!(
                    tool = self.tool.context.as_str(),
                    script = ?abs_script,
                    "Post-install script is not executable",
                );
            } else {
                debug!(
                    tool = self.tool.context.as_str(),
                    script = ?abs_script,
                    "Executing post-install script",
                );

                let shell = ShellType::detect_with_fallback().build();
                let mut command = Command::from(shell.create_wrapped_command_with(join_exe_args(
                    &shell,
                    &abs_script,
                    &output.post_script_args,
                    false,
                )));
                command.current_dir(&self.product_dir);

                proto.create_metric().record_tool_install_step(
                    &self.tool.context,
                    "post_install_script",
                    process::exec_command(&mut command).await,
                )?;
            }
        }

        Ok(record)
    }

    /// Uninstall the tool by deleting the current install directory.
    #[instrument(skip(self))]
    pub async fn uninstall(&self) -> Result<bool, ProtoInstallError> {
        if !self.product_dir.exists() {
            debug!(
                tool = self.tool.context.as_str(),
                "Tool has not been installed, aborting"
            );

            return Ok(false);
        }

        if self
            .tool
            .plugin
            .has_func(PluginFunction::NativeUninstall)
            .await
        {
            debug!(
                tool = self.tool.context.as_str(),
                "Uninstalling tool natively"
            );

            let output: NativeUninstallOutput = self
                .tool
                .plugin
                .call_func_with(
                    PluginFunction::NativeUninstall,
                    NativeUninstallInput {
                        context: self.tool.create_plugin_context(self.spec),
                        uninstall_dir: self.tool.to_virtual_path(&self.product_dir),
                    },
                )
                .await?;

            if !output.uninstalled && !output.skip_uninstall {
                return Err(ProtoInstallError::FailedUninstall {
                    tool: self.tool.get_name().to_owned(),
                    error: output.error.unwrap_or_default(),
                });
            }
        }

        debug!(
            tool = self.tool.context.as_str(),
            install_dir = ?self.product_dir,
            "Deleting install directory"
        );

        fs::remove_dir_all(&self.product_dir)?;

        debug!(
            tool = self.tool.context.as_str(),
            "Successfully uninstalled tool"
        );

        Ok(true)
    }

    /// Verify the downloaded file using the checksum strategy for the tool.
    /// Common strategies are SHA256 and MD5.
    #[instrument(skip(self))]
    pub async fn verify_checksum(
        &self,
        checksum_file: &Path,
        download_file: &Path,
        checksum_public_key: Option<&str>,
    ) -> Result<Checksum, ProtoInstallError> {
        debug!(
            tool = self.tool.context.as_str(),
            download_file = ?download_file,
            checksum_file = ?checksum_file,
            "Verifying checksum of downloaded file",
        );

        let proto = &self.tool.proto;

        let checksum = proto.create_metric().record_tool_install_step(
            &self.tool.context,
            "generate_checksum",
            generate_checksum(download_file, checksum_file, checksum_public_key),
        )?;

        // Allow plugin to provide their own checksum verification method

        let verified = if self
            .tool
            .plugin
            .has_func(PluginFunction::VerifyChecksum)
            .await
        {
            let output: VerifyChecksumOutput = proto.create_metric().record_tool_install_step(
                &self.tool.context,
                "verify_checksum",
                self.tool
                    .plugin
                    .call_func_with(
                        PluginFunction::VerifyChecksum,
                        VerifyChecksumInput {
                            checksum_file: self.tool.to_virtual_path(checksum_file),
                            download_file: self.tool.to_virtual_path(download_file),
                            download_checksum: Some(checksum.clone()),
                            context: self.tool.create_plugin_context(self.spec),
                        },
                    )
                    .await,
            )?;

            output.verified
        }
        // Otherwise attempt to verify it ourselves
        else {
            proto.create_metric().record_tool_install_step(
                &self.tool.context,
                "verify_checksum",
                verify_checksum(download_file, checksum_file, &checksum),
            )?
        };

        if verified {
            debug!(
                tool = self.tool.context.as_str(),
                "Successfully verified, checksum matches"
            );

            return Ok(checksum);
        }

        Err(ProtoInstallError::InvalidChecksum {
            hash: checksum.to_string(),
            checksum: checksum_file.to_path_buf(),
            download: download_file.to_path_buf(),
        })
    }
}
