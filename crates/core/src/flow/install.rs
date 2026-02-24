use super::build::*;
pub use super::build_error::ProtoBuildError;
pub use super::install_error::ProtoInstallError;
use crate::checksum::*;
use crate::env::ProtoConsole;
use crate::flow::lock::Locker;
use crate::helpers::{extract_filename_from_url, is_archive_file, is_executable, is_offline};
use crate::lockfile::*;
use crate::tool::Tool;
use crate::tool_spec::ToolSpec;
use crate::utils::log::LogWriter;
use crate::utils::{archive, process};
use proto_pdk_api::*;
use starbase_shell::ShellType;
use starbase_styles::color;
use starbase_utils::net::DownloadOptions;
use starbase_utils::{fs, net, path};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use system_env::System;
use tokio::process::Command;
use tracing::{debug, instrument, warn};

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

#[derive(Default)]
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
            temp_dir: tool.get_temp_dir().to_path_buf(),
            tool,
            spec,
        }
    }

    /// Install a tool into proto, either by downloading and unpacking
    /// a pre-built archive, or by using a native installation method.
    #[instrument(skip(self, options))]
    pub async fn install(
        &self,
        options: InstallOptions,
    ) -> Result<Option<LockRecord>, ProtoInstallError> {
        if self.tool.is_installed(self.spec) && !options.force {
            debug!(
                tool = self.tool.context.as_str(),
                "Tool already installed, continuing"
            );

            return Ok(None);
        }

        if is_offline() {
            return Err(ProtoInstallError::RequiredInternetConnection);
        }

        // Lock the temporary directory instead of the install directory,
        // because the latter needs to be clean for "build from source",
        // and the `.lock` file breaks that contract
        let mut install_lock = fs::lock_directory(&self.temp_dir)?;

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

            let output: NativeInstallOutput = self
                .tool
                .plugin
                .call_func_with(
                    PluginFunction::NativeInstall,
                    NativeInstallInput {
                        context: self.tool.create_plugin_context(self.spec),
                        install_dir: self.tool.to_virtual_path(&self.product_dir),
                        force: options.force,
                    },
                )
                .await?;

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

        match result {
            Ok(record) => {
                // Verify against lockfile
                Locker::new(self.tool).verify_locked_record(self.spec, &record)?;

                debug!(
                    tool = self.tool.context.as_str(),
                    install_dir = ?self.product_dir,
                    "Successfully installed tool",
                );

                Ok(Some(record))
            }

            // Clean up if the install failed
            Err(error) => {
                debug!(
                    tool = self.tool.context.as_str(),
                    install_dir = ?self.product_dir,
                    "Failed to install tool, cleaning up",
                );

                install_lock.unlock()?;

                fs::remove_dir_all(&self.product_dir)?;
                fs::remove_dir_all(&self.temp_dir)?;

                Err(error)
            }
        }
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

        let output: BuildInstructionsOutput = self
            .tool
            .plugin
            .cache_func_with(
                PluginFunction::BuildInstructions,
                BuildInstructionsInput {
                    context: self.tool.create_plugin_context(self.spec),
                    install_dir: self.tool.to_virtual_path(&self.product_dir),
                },
            )
            .await?;

        let mut system = System::default();
        let config = self.tool.proto.load_config()?;

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

        let mut builder = Builder::new(BuilderOptions {
            config,
            console: options
                .console
                .as_ref()
                .expect("Console required for builder!"),
            install_dir: &self.product_dir,
            http_client: self.tool.proto.get_plugin_loader()?.get_http_client()?,
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
        });

        // The build process may require using itself to build itself,
        // so allow proto to use any available version instead of failing
        unsafe { std::env::set_var(format!("{}_VERSION", self.tool.get_env_var_prefix()), "*") };

        let mut record = self.tool.create_locked_record();

        // Step 0
        log_build_information(&mut builder, &output)?;

        // Step 1
        if config.settings.build.install_system_packages {
            install_system_dependencies(&mut builder, &output).await?;
        } else {
            debug!(
                tool = self.tool.context.as_str(),
                "Not installing system dependencies because {} was disabled",
                color::property("settings.build.install-system-packages"),
            );
        }

        // Step 2
        check_requirements(&mut builder, &output).await?;

        // Step 3
        download_sources(&mut builder, &output, &mut record).await?;

        // Step 4
        execute_instructions(&mut builder, &output, &self.tool.proto).await?;

        Ok(record)
    }

    /// Download the tool (as an archive) from its distribution registry
    /// into the `~/.proto/tools/<version>` folder, and optionally verify checksums.
    #[instrument(skip(self, options))]
    async fn install_from_prebuilt(
        &self,
        options: InstallOptions,
    ) -> Result<LockRecord, ProtoInstallError> {
        debug!(
            tool = self.tool.context.as_str(),
            "Installing tool by downloading a pre-built archive"
        );

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

        let client = self.tool.proto.get_plugin_loader()?.get_http_client()?;
        let config = self.tool.proto.load_config()?;

        let output: DownloadPrebuiltOutput = self
            .tool
            .plugin
            .cache_func_with(
                PluginFunction::DownloadPrebuilt,
                DownloadPrebuiltInput {
                    context: self.tool.create_plugin_context(self.spec),
                    install_dir: self.tool.to_virtual_path(&self.product_dir),
                },
            )
            .await?;

        let mut record = self.tool.create_locked_record();

        // Download the prebuilt
        let download_url = config.rewrite_url(output.download_url);
        let download_filename = match output.download_name {
            Some(name) => name,
            None => extract_filename_from_url(&download_url),
        };
        let download_file = self.temp_dir.join(&download_filename);

        record.source = Some(download_url.clone());
        options.on_phase_change.as_ref().inspect(|func| {
            func(InstallPhase::Download {
                url: download_url.clone(),
                file: download_filename.clone(),
            });
        });

        debug!(
            tool = self.tool.context.as_str(),
            "Downloading tool archive"
        );

        net::download_from_url_with_options(
            &download_url,
            &download_file,
            DownloadOptions {
                downloader: Some(Box::new(client.create_downloader())),
                on_chunk: options.on_download_chunk.clone(),
            },
        )
        .await?;

        // Verify against a URL that contains the checksum
        if let Some(checksum_url) = output.checksum_url {
            let checksum_url = config.rewrite_url(checksum_url);
            let checksum_filename = match output.checksum_name {
                Some(name) => name,
                None => extract_filename_from_url(&checksum_url),
            };
            let checksum_file = self.temp_dir.join(&checksum_filename);

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
            let checksum_file = self
                .temp_dir
                .join(format!("CHECKSUM.{:?}", checksum.algo).to_lowercase());

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
            record.checksum = Some(Checksum::sha256(hash_file_contents_sha256(&download_file)?));
        }

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
                    file: download_filename.clone(),
                });
            });

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
                &self.product_dir,
                &download_file,
                output.archive_prefix.as_deref(),
            )?;

            // If the archive was `.gz` without tar or other formats,
            // it's a single file, so assume a file and update perms
            if ext == "gz" && unpacked_path.is_file() {
                fs::update_perms(unpacked_path, None)?;
            }
        }
        // Not an archive, assume a file and copy
        else {
            let install_path = self.product_dir.join(path::exe_name(path::encode_component(
                self.tool.get_file_name(),
            )));

            fs::rename(&download_file, &install_path)?;
            fs::update_perms(install_path, None)?;
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
            } else if !is_executable(&abs_script) {
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
                let mut command = Command::new(shell.to_string());
                command.arg("-c");
                command.arg(shell.quote(&abs_script.to_string_lossy()));
                command.current_dir(&self.product_dir);

                process::exec_command(&mut command).await?;
            }
        }

        Ok(record)
    }

    /// Uninstall the tool by deleting the current install directory.
    #[instrument(skip_all)]
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

        let checksum = generate_checksum(download_file, checksum_file, checksum_public_key)?;
        let verified;

        // Allow plugin to provide their own checksum verification method
        if self
            .tool
            .plugin
            .has_func(PluginFunction::VerifyChecksum)
            .await
        {
            let output: VerifyChecksumOutput = self
                .tool
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
                .await?;

            verified = output.verified;
        }
        // Otherwise attempt to verify it ourselves
        else {
            verified = verify_checksum(download_file, checksum_file, &checksum)?;
        }

        if verified {
            debug!(
                tool = self.tool.context.as_str(),
                "Successfully verified, checksum matches"
            );

            return Ok(checksum);
        }

        Err(ProtoInstallError::InvalidChecksum {
            checksum: checksum_file.to_path_buf(),
            download: download_file.to_path_buf(),
        })
    }
}
