use super::build_error::*;
use super::install::{InstallPhase, OnPhaseFn};
use crate::config::ProtoConfig;
use crate::env::ProtoEnvironment;
use crate::id::Id;
use crate::lockfile::LockRecord;
use crate::reporter::ProtoConsole;
use crate::tool::Tool;
use crate::utils::log::LogWriter;
use crate::utils::process::{self, ProcessResult, ProtoProcessError};
use crate::utils::{archive, git};
use iocraft::prelude::{FlexDirection, View, element};
use proto_pdk_api::{
    BuildInstruction, BuildInstructionsOutput, BuildRequirement, GitSource, SourceLocation,
};
use rustc_hash::FxHashMap;
use starbase_console::ConsoleError;
use starbase_console::ui::{
    Confirm, Container, Entry, ListCheck, ListItem, Section, Select, SelectOption, Style,
    StyledText,
};
use starbase_styles::{apply_style_tags, color, remove_style_tags};
use starbase_utils::fs::LOCK_FILE;
use starbase_utils::net::DownloadOptions;
use starbase_utils::{envx::is_ci, fs, net, path};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use system_env::{
    DependencyConfig, DependencyName, System, SystemPackageManager, find_command_on_path,
    is_command_on_path,
};
use tokio::process::Command;
use tokio::sync::{Mutex, OwnedMutexGuard};
use tracing::{debug, error, instrument};
use version_spec::{MatchesVersion, Requirement, Version, VersionSpec};
use warpgate::{HttpClient, extract_file_name_from_url};

static BUILD_LOCKS: OnceLock<scc::HashMap<String, Arc<Mutex<()>>>> = OnceLock::new();

pub struct BuilderOptions<'a> {
    pub config: &'a ProtoConfig,
    pub console: Option<&'a ProtoConsole>,
    pub http_client: &'a HttpClient,
    pub install_dir: &'a Path,
    pub log_writer: &'a LogWriter,
    pub on_phase_change: Option<OnPhaseFn>,
    pub skip_prompts: bool,
    pub skip_ui: bool,
    pub system: System,
    pub temp_dir: &'a Path,
    pub version: VersionSpec,
}

pub struct Builder<'tool> {
    pub options: BuilderOptions<'tool>,
    errors: u8,
    tool: &'tool Tool,
}

impl<'tool> Builder<'tool> {
    pub fn new(tool: &'tool Tool, options: BuilderOptions<'tool>) -> Self {
        Builder {
            errors: 0,
            options,
            tool,
        }
    }

    pub async fn build(
        mut self,
        output: BuildInstructionsOutput,
    ) -> Result<LockRecord, ProtoBuildError> {
        let context = &self.tool.context;
        let proto = &self.tool.proto;
        let mut record = self.tool.create_locked_record();

        // The build process may require using itself to build itself,
        // so allow proto to use any available version instead of failing
        unsafe { std::env::set_var(format!("{}_VERSION", self.tool.get_env_var_prefix()), "*") };

        // Step 0
        proto.create_metric().record_tool_install_step(
            context,
            "build_information",
            self.log_build_information(&output),
        )?;

        // Step 1
        if self.options.config.settings.build.install_system_packages {
            proto.create_metric().record_tool_install_step(
                context,
                "install_system_dependencies",
                self.install_system_dependencies(&output).await,
            )?;
        } else {
            debug!(
                tool = context.as_str(),
                "Not installing system dependencies because {} was disabled",
                color::property("settings.build.install-system-packages"),
            );
        }

        // Step 2
        proto.create_metric().record_tool_install_step(
            context,
            "check_requirements",
            self.check_requirements(&output).await,
        )?;

        // Step 3
        proto.create_metric().record_tool_install_step(
            context,
            "download_sources",
            self.download_sources(&output, &mut record).await,
        )?;

        // Step 4
        proto.create_metric().record_tool_install_step(
            context,
            "execute_instructions",
            self.execute_instructions(&output, proto).await,
        )?;

        Ok(record)
    }

    pub fn get_system(&self) -> &System {
        &self.options.system
    }

    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }

    pub fn render_header(&mut self, title: impl AsRef<str>) -> Result<(), ConsoleError> {
        let context = &self.tool.context;
        let title = title.as_ref();

        self.errors = 0;
        self.options.log_writer.add_header(title);

        if self.options.skip_ui || self.options.console.is_none() {
            debug!(tool = context.as_str(), "{}", apply_style_tags(title));
        } else if let Some(console) = self.options.console {
            if console.is_json_format() {
                console.progress(title, Some(context.to_string()))?;
            } else {
                console.out.write_newline()?;
                console.render(element! {
                    Container {
                        Section(title)
                    }
                })?;
            }
        }

        Ok(())
    }

    pub fn render_check(
        &mut self,
        message: impl AsRef<str>,
        passed: bool,
    ) -> Result<(), ConsoleError> {
        let context = &self.tool.context;
        let message = message.as_ref();

        if self.options.skip_ui || self.options.console.is_none() {
            let message = apply_style_tags(message);

            if passed {
                debug!(tool = context.as_str(), "{message}");
            } else {
                error!(tool = context.as_str(), "{message}");
            }
        } else if let Some(console) = self.options.console {
            if console.is_json_format() {
                console.message(message)?;
            } else {
                console.render(element! {
                    ListCheck(checked: passed) {
                        StyledText(content: message)
                    }
                })?;
            }
        }

        if !passed {
            self.errors += 1;
        }

        Ok(())
    }

    pub fn render_checkpoint(&mut self, message: impl AsRef<str>) -> Result<(), ConsoleError> {
        let context = &self.tool.context;
        let message = message.as_ref();

        self.options
            .log_writer
            .add_section(remove_style_tags(message));

        if self.options.skip_ui || self.options.console.is_none() {
            debug!(tool = context.as_str(), "{}", apply_style_tags(message));
        } else if let Some(console) = self.options.console {
            if console.is_json_format() {
                console.progress(message, Some(context.to_string()))?;
            } else {
                console.render(element! {
                    ListItem(bullet: "❯".to_owned()) {
                        StyledText(content: message)
                    }
                })?;
            }
        }

        Ok(())
    }

    pub async fn prompt_continue(&self, label: &str) -> Result<(), ProtoBuildError> {
        if self.options.skip_prompts || self.options.skip_ui || self.options.console.is_none() {
            return Ok(());
        }

        let mut confirmed = false;

        self.options
            .console
            .unwrap()
            .render_interactive(element! {
                Confirm(label, on_confirm: &mut confirmed)
            })
            .await?;

        if !confirmed {
            return Err(ProtoBuildError::Cancelled);
        }

        Ok(())
    }

    pub async fn prompt_select(
        &self,
        label: &str,
        options: Vec<SelectOption>,
        default_index: usize,
    ) -> Result<usize, ProtoBuildError> {
        let mut selected_index = default_index;

        if self.options.skip_prompts || self.options.skip_ui || self.options.console.is_none() {
            return Ok(selected_index);
        }

        self.options
            .console
            .unwrap()
            .render_interactive(element! {
                Select(label, options, default_index, on_index: &mut selected_index)
            })
            .await?;

        Ok(selected_index)
    }

    pub async fn exec_command(
        &mut self,
        command: &mut Command,
        piped: bool,
    ) -> Result<Arc<ProcessResult>, ProtoProcessError> {
        self.handle_process_result(if self.options.skip_ui || piped {
            process::exec_command_piped(command).await?
        } else {
            process::exec_command(command).await?
        })
    }

    pub async fn exec_command_with_privileges(
        &mut self,
        command: &mut Command,
        elevated_program: Option<&str>,
        piped: bool,
    ) -> Result<Arc<ProcessResult>, ProtoProcessError> {
        self.handle_process_result(if self.options.skip_ui || piped {
            process::exec_command_with_privileges_piped(command, elevated_program).await?
        } else {
            process::exec_command_with_privileges(command, elevated_program).await?
        })
    }

    fn handle_process_result(
        &mut self,
        result: ProcessResult,
    ) -> Result<Arc<ProcessResult>, ProtoProcessError> {
        let result = Arc::new(result);

        let log = self.options.log_writer;
        log.add_subsection(format!("Child process: `{}`", result.command));
        log.add_value_opt(
            "WORKING DIR",
            result.working_dir.as_ref().and_then(|dir| dir.to_str()),
        );
        log.add_value("EXIT CODE", result.exit_code.to_string());
        log.add_code("STDERR", &result.stderr);
        log.add_code("STDOUT", &result.stdout);

        if result.exit_code != 0 {
            return Err(ProtoProcessError::FailedCommandNonZeroExit {
                command: result.command.clone(),
                code: result.exit_code,
                stderr: result.stderr.clone(),
            });
        }

        Ok(result)
    }

    pub async fn acquire_lock(&self, pm: &SystemPackageManager) -> OwnedMutexGuard<()> {
        let locks = BUILD_LOCKS.get_or_init(scc::HashMap::default);
        let entry = locks.entry_async(pm.to_string()).await.or_default();

        entry.get().clone().lock_owned().await
    }
}

#[instrument(skip(builder))]
async fn checkout_git_repo(
    git: &GitSource,
    cwd: &Path,
    builder: &mut Builder<'_>,
) -> Result<(), ProtoBuildError> {
    if cwd.join(".git").exists() {
        builder.exec_command(&mut git::new_pull(cwd), false).await?;

        return Ok(());
    }

    fs::create_dir_all(cwd)?;

    builder
        .exec_command(&mut git::new_clone(git, cwd), false)
        .await?;

    if let Some(reference) = &git.reference {
        builder.render_checkpoint(format!("Checking out reference <hash>{reference}</hash>"))?;

        builder
            .exec_command(&mut git::new_checkout(reference, cwd), false)
            .await?;
    }

    Ok(())
}

// STEP 0

impl Builder<'_> {
    fn log_build_information(
        &self,
        build: &BuildInstructionsOutput,
    ) -> Result<(), ProtoBuildError> {
        let system = &self.options.system;

        if self.options.skip_ui || self.options.console.is_none_or(|c| c.is_json_format()) {
            debug!(
                tool = self.tool.context.as_str(),
                os = ?system.os,
                arch = ?system.arch,
                pm = ?system.manager,
                version = self.options.version.to_string(),
                "Gathering system information",
            );
        } else if let Some(console) = self.options.console {
            console.render(element! {
                Container {
                    Section(title: "Build information")
                    View(padding_left: 2, flex_direction: FlexDirection::Column) {
                        Entry(name: "Operating system", content: system.os.to_string())
                        Entry(name: "Architecture", content: system.arch.to_string())
                        #(system.manager.map(|pm| {
                            element! {
                                Entry(name: "Package manager", content: pm.to_string())
                            }
                        }))
                        Entry(name: "Target version", value: element! {
                            StyledText(content: self.options.version.to_string(), style: Style::Hash)
                        }.into_any())
                        #(build.help_url.as_ref().map(|url| {
                            element! {
                                Entry(name: "Documentation", value: element! {
                                    StyledText(content: url, style: Style::Url)
                                }.into_any())
                            }
                        }))
                    }
                }
            })?;
        }

        Ok(())
    }
}

// STEP 1

enum InstallSystemDepsChoice {
    NoAndAbort = 0,
    NoButContinue = 1,
    Yes = 2,
    YesButElevated = 3,
}

impl Builder<'_> {
    #[instrument(skip(self))]
    async fn install_system_dependencies(
        &mut self,
        build: &BuildInstructionsOutput,
    ) -> Result<(), ProtoBuildError> {
        let Some(pm) = self.options.system.manager else {
            return Ok(());
        };

        // Determine packages to install
        let pm_config = pm.get_config();
        let dep_configs = self
            .get_system()
            .resolve_dependencies(&build.system_dependencies);

        // 1) Check if packages have already been installed
        let mut not_installed_packages = FxHashMap::from_iter(
            dep_configs
                .iter()
                .filter_map(|cfg| cfg.get_package_names_and_versions(&pm).ok())
                .flatten(),
        );

        for excluded in &self.options.config.settings.build.exclude_packages {
            not_installed_packages.remove(excluded);
        }

        if not_installed_packages.is_empty() {
            return Ok(());
        }

        self.render_header("Installing system dependencies")?;

        self.options.on_phase_change.as_ref().inspect(|func| {
            func(InstallPhase::InstallDeps);
        });

        if let Ok(Some(mut list_args)) = self
            .get_system()
            .get_list_packages_command(!self.options.skip_prompts)
        {
            let _lock = self.acquire_lock(&pm).await;

            self.render_checkpoint(format!("Checking <shell>{pm}</shell> installed packages"))?;

            let list_output = self
                .exec_command(Command::new(list_args.remove(0)).args(list_args), true)
                .await?;
            let installed_packages = pm_config.list_parser.parse(&list_output.stdout);
            let mut skipped_packages = FxHashMap::default();

            not_installed_packages.retain(|name, constraint| {
                let retained = match (
                    constraint.as_ref(),
                    installed_packages.get(name).and_then(|con| con.as_ref()),
                ) {
                    (Some(required_version), Some(installed_version)) => {
                        if let (Ok(req), Ok(ver)) = (
                            Requirement::parse(required_version),
                            Version::parse(installed_version),
                        ) {
                            // Doesn't match, so we need to install
                            !req.matches(&ver)
                        } else {
                            // Unable to parse, so install just in case
                            true
                        }
                    }

                    // Not enough information, so just check if installed
                    _ => !installed_packages.contains_key(name),
                };

                if !retained {
                    skipped_packages.insert(name.clone(), constraint.clone());
                }

                retained
            });

            // Print packages that are already installed
            for (package, version) in skipped_packages {
                self.render_check(
                    match version {
                        Some(version) => {
                            format!("<id>{package}</id> v{version} already installed")
                        }
                        None => format!("<id>{package}</id> already installed"),
                    },
                    true,
                )?;
            }
        }

        // Print the packages that are not installed
        for (package, version) in &not_installed_packages {
            self.render_check(
                match version {
                    Some(version) => {
                        format!("<id>{package}</id> v{version} is not installed")
                    }
                    None => format!("<id>{package}</id> is not installed"),
                },
                false,
            )?;
        }

        if not_installed_packages.is_empty() {
            return Ok(());
        }

        // 2) Prompt the user to choose an install strategy
        let mut elevated_command = pm.get_elevated_command();
        let mut select_options = vec![
            SelectOption::new("No, and stop building"),
            SelectOption::new("No, but try building anyways"),
            SelectOption::new("Yes, as current user"),
        ];

        // When installing multiple tools, we can't prompt the user to install
        // deps, but we should try to build anyways
        let mut default_index = if self.options.skip_ui {
            InstallSystemDepsChoice::NoButContinue
        } else {
            InstallSystemDepsChoice::Yes
        };

        if let Some(sudo) = elevated_command {
            select_options.push(SelectOption::new(format!(
                "Yes, with elevated privileges ({sudo})"
            )));

            // Always run with elevated in CI
            if is_ci() {
                default_index = InstallSystemDepsChoice::YesButElevated;
            }
        }

        match self
            .prompt_select(
                "Install missing packages?",
                select_options,
                default_index as usize,
            )
            .await?
        {
            x if x == InstallSystemDepsChoice::NoAndAbort as usize => {
                return Err(ProtoBuildError::Cancelled);
            }
            x if x == InstallSystemDepsChoice::NoButContinue as usize => {
                return Ok(());
            }
            x if x == InstallSystemDepsChoice::Yes as usize => {
                elevated_command = None;
            }
            _ => {}
        }

        // 3) Update the current registry index
        if let Some(mut index_args) = self
            .get_system()
            .get_update_index_command(!self.options.skip_prompts)?
        {
            let _lock = self.acquire_lock(&pm).await;

            self.render_checkpoint("Updating package manager index")?;

            self.exec_command_with_privileges(
                Command::new(index_args.remove(0)).args(index_args),
                elevated_command,
                false,
            )
            .await?;
        }

        // Recreate the dep configs since they've been filtered
        let dep_configs = not_installed_packages
            .into_iter()
            .map(|(name, version)| DependencyConfig {
                dep: DependencyName::Single(name),
                version,
                ..Default::default()
            })
            .collect::<Vec<_>>();

        // 4) Install the missing packages
        if let Some(mut install_args) = self
            .get_system()
            .get_install_packages_command(&dep_configs, !self.options.skip_prompts)?
        {
            let _lock = self.acquire_lock(&pm).await;

            self.render_checkpoint(format!("Installing <shell>{pm}</shell> packages"))?;

            self.exec_command_with_privileges(
                Command::new(install_args.remove(0)).args(install_args),
                elevated_command,
                false,
            )
            .await?;
        }

        Ok(())
    }
}

// STEP 2

fn get_command_version_regex() -> &'static regex::Regex {
    static VERSION_REGEX: OnceLock<regex::Regex> = OnceLock::new();

    VERSION_REGEX.get_or_init(|| regex::Regex::new(r"[0-9]+\.[0-9]+\.[0-9]+").unwrap())
}

impl Builder<'_> {
    #[instrument(skip(self))]
    async fn get_command_version(
        &mut self,
        cmd: &str,
        version_arg: &str,
    ) -> Result<Version, ProtoBuildError> {
        let output = self
            .exec_command(Command::new(cmd).arg(version_arg), true)
            .await?;

        let value = get_command_version_regex()
            .find(&output.stdout)
            .map(|res| res.as_str())
            .unwrap_or(&output.stdout);

        Version::parse(value).map_err(|error| ProtoBuildError::FailedVersionParse {
            value: value.to_owned(),
            error: Box::new(error),
        })
    }

    #[instrument(skip(self))]
    async fn check_requirements(
        &mut self,
        build: &BuildInstructionsOutput,
    ) -> Result<(), ProtoBuildError> {
        if build.requirements.is_empty() {
            return Ok(());
        }

        self.render_header("Checking requirements")?;

        self.options.on_phase_change.as_ref().inspect(|func| {
            func(InstallPhase::CheckRequirements);
        });

        for req in &build.requirements {
            match req {
                BuildRequirement::CommandExistsOnPath(cmd) => {
                    debug!(cmd, "Checking if a command exists on PATH");

                    if let Some(cmd_path) = find_command_on_path(cmd) {
                        self.render_check(
                            format!(
                                "Command <shell>{cmd}</shell> exists on PATH: <path>{}</path>",
                                cmd_path.display()
                            ),
                            true,
                        )?;
                    } else {
                        self.render_check(
                            format!("Command <shell>{cmd}</shell> does NOT exist on PATH, please install it and try again"),
                            false,
                        )?;
                    }
                }
                BuildRequirement::CommandVersion(cmd, version_req, version_arg) => {
                    debug!(
                        cmd,
                        "Checking if a command meets the required version of {version_req}"
                    );

                    if is_command_on_path(cmd) {
                        let version = self
                            .get_command_version(cmd, version_arg.as_deref().unwrap_or("--version"))
                            .await?;

                        if version_req.matches(&version) {
                            self.render_check(
                                format!("Command <shell>{cmd}</shell> meets the minimum required version of {version_req}"),
                                true,
                            )?;
                        } else {
                            self.render_check(
                                format!("Command <shell>{cmd}</shell> does NOT meet the minimum required version of {version_req}, found {version}"),
                                false,
                            )?;
                        }
                    } else {
                        self.render_check(
                            format!("Command <shell>{cmd}</shell> does NOT exist on PATH, please install it and try again"),
                            false,
                        )?;
                    }
                }
                BuildRequirement::ManualIntercept(url) => {
                    self.render_check(
                        format!("Please read the following documentation before proceeding: <url>{url}</url>"),
                        true,
                    )?;

                    self.prompt_continue("Continue install?").await?;
                }
                BuildRequirement::GitConfigSetting(config_key, expected_value) => {
                    debug!(
                        config_key,
                        expected_value, "Checking if a Git config setting has the expected value"
                    );

                    let result = self
                        .exec_command(
                            Command::new("git").args(["config", "--get", config_key]),
                            true,
                        )
                        .await?;
                    let actual_value = &result.stdout;

                    if actual_value == expected_value {
                        self.render_check(
                            format!("Git config <property>{config_key}</property> matches the required value of <symbol>{expected_value}</symbol>"),
                            true,
                        )?;
                    } else {
                        self.render_check(
                            format!("Git config <property>{config_key}</property> does NOT match the required value or <symbol>{expected_value}</symbol>, found {actual_value}"),
                            false,
                        )?;
                    }
                }
                BuildRequirement::GitVersion(version_req) => {
                    debug!("Checking if Git meets the required version of {version_req}");

                    let version = self.get_command_version("git", "--version").await?;

                    if version_req.matches(&version) {
                        self.render_check(
                            format!("Git meets the minimum required version of {version_req}"),
                            true,
                        )?;
                    } else {
                        self.render_check(
                            format!("Git does NOT meet the minimum required version of {version_req}, found {version}"),
                            false,
                        )?;
                    }
                }
                BuildRequirement::XcodeCommandLineTools => {
                    if self.get_system().os.is_mac() {
                        debug!("Checking if Xcode command line tools are installed");

                        let result = self
                            .exec_command(Command::new("xcode-select").arg("--version"), true)
                            .await;

                        if result.is_err() || result.is_ok_and(|out| out.stdout.is_empty()) {
                            self.render_check(
                                "Xcode command line tools are NOT installed, install them with <shell>xcode-select --install</shell>",
                                false,
                            )?;
                        } else {
                            self.render_check("Xcode command line tools are installed", true)?;
                        }
                    }
                }
                BuildRequirement::WindowsDeveloperMode => {
                    if self.get_system().os.is_windows() {
                        debug!("Checking if Windows developer mode is enabled");

                        // Is this possible from the command line?
                    }
                }
            };
        }

        if self.has_errors() {
            return Err(ProtoBuildError::RequirementsNotMet);
        }

        Ok(())
    }
}

// STEP 3

impl Builder<'_> {
    #[instrument(skip(self))]
    async fn download_sources(
        &mut self,
        build: &BuildInstructionsOutput,
        lockfile: &mut LockRecord,
    ) -> Result<(), ProtoBuildError> {
        // Ensure the install directory is empty, otherwise Git will fail and
        // we also want to avoid colliding/stale artifacts. This should also
        // run if there's no source, as it's required for instructions!
        fs::remove_dir_all(self.options.install_dir)?;
        fs::create_dir_all(self.options.install_dir)?;

        let Some(source) = &build.source else {
            return Ok(());
        };

        self.render_header("Acquiring source files")?;

        match source {
            SourceLocation::Archive(archive) => {
                let mut archive = archive.to_owned();
                archive.url = self.options.config.rewrite_url(archive.url);

                lockfile.source = Some(archive.url.clone());

                if archive::should_unpack(&archive, self.options.install_dir)? {
                    let filename = extract_file_name_from_url(&archive.url);

                    // Download
                    self.options.on_phase_change.as_ref().inspect(|func| {
                        func(InstallPhase::Download {
                            url: archive.url.clone(),
                            file: filename.clone(),
                        });
                    });

                    self.render_checkpoint(format!(
                        "Downloading archive from <url>{}</url>",
                        archive.url
                    ))?;

                    let download_file = archive::download(
                        &archive,
                        self.options.temp_dir,
                        DownloadOptions::new(
                            self.options
                                .http_client
                                .create_downloader_with_headers(build.http_headers.clone()),
                        ),
                    )
                    .await?;

                    // Unpack
                    self.options.on_phase_change.as_ref().inspect(|func| {
                        func(InstallPhase::Unpack {
                            file: filename.clone(),
                        });
                    });

                    self.render_checkpoint(format!(
                        "Unpacking archive to <path>{}</path>",
                        self.options.install_dir.display()
                    ))?;

                    archive::unpack_source(
                        &archive,
                        self.options.install_dir,
                        self.options.temp_dir,
                        &download_file,
                    )
                    .await?;
                }
            }
            SourceLocation::Git(git) => {
                lockfile.source = Some(match &git.reference {
                    Some(rev) => format!("{}#{rev}", git.url),
                    None => git.url.clone(),
                });

                self.options.on_phase_change.as_ref().inspect(|func| {
                    func(InstallPhase::CloneRepository {
                        url: git.url.clone(),
                    });
                });

                self.render_checkpoint(format!("Cloning repository <url>{}</url>", git.url))?;

                checkout_git_repo(git, self.options.install_dir, self).await?;
            }
        };

        Ok(())
    }
}

// STEP 4

impl Builder<'_> {
    #[instrument(skip(self))]
    async fn execute_instructions(
        &mut self,
        build: &BuildInstructionsOutput,
        proto: &ProtoEnvironment,
    ) -> Result<(), ProtoBuildError> {
        if build.instructions.is_empty() {
            return Ok(());
        }

        self.render_header("Executing build instructions")?;

        self.options.on_phase_change.as_ref().inspect(|func| {
            func(InstallPhase::ExecuteInstructions);
        });

        let make_absolute = |path: &Path| {
            if path.is_absolute() {
                PathBuf::from(path::normalize_separators(path))
            } else {
                self.options
                    .install_dir
                    .join(path::normalize_separators(path))
            }
        };

        let total = build.instructions.len();
        let mut builder_exes = FxHashMap::default();

        for (index, instruction) in build.instructions.iter().enumerate() {
            debug!("Executing build instruction {} of {total}", index + 1);

            let prefix = format!("<mutedlight>[{}/{total}]</mutedlight>", index + 1);

            match instruction {
                BuildInstruction::InstallBuilder(item) => {
                    self.render_checkpoint(format!(
                        "{prefix} Installing <id>{}</id> builder (<url>{}<url>)",
                        item.id, item.git.url
                    ))?;

                    let builder_dir = proto.store.builders_dir.join(item.id.as_str());

                    checkout_git_repo(&item.git, &builder_dir, self).await?;

                    let main_exe_name = String::new();
                    let mut exes = FxHashMap::default();
                    exes.extend(&item.exes);
                    exes.insert(&main_exe_name, &item.exe);

                    for (exe_name, exe_rel_path) in exes {
                        let exe_abs_path =
                            builder_dir.join(path::normalize_separators(exe_rel_path));

                        if !exe_abs_path.exists() {
                            return Err(ProtoBuildError::MissingBuilderExe {
                                exe: exe_abs_path,
                                id: item.id.clone(),
                            });
                        }

                        if !fs::is_executable(&exe_abs_path) {
                            fs::update_perms(&exe_abs_path, None)?;
                        }

                        builder_exes.insert(
                            if exe_name.is_empty() {
                                item.id.to_string()
                            } else {
                                format!("{}:{exe_name}", item.id)
                            },
                            exe_abs_path,
                        );
                    }
                }
                BuildInstruction::MakeExecutable(file) => {
                    let file = make_absolute(file);

                    self.render_checkpoint(format!(
                        "{prefix} Making file <path>{}</path> executable",
                        file.display()
                    ))?;

                    fs::update_perms(file, None)?;
                }
                BuildInstruction::MoveFile(from, to) => {
                    let from = make_absolute(from);
                    let to = make_absolute(to);

                    self.render_checkpoint(format!(
                        "{prefix} Moving <path>{}</path> to <path>{}</path>",
                        from.display(),
                        to.display(),
                    ))?;

                    fs::rename(from, to)?;
                }
                BuildInstruction::RemoveAllExcept(exceptions) => {
                    let dir = self.options.install_dir;

                    self.render_checkpoint(format!(
                        "{prefix} Removing directory <path>{}</path> except for {}",
                        dir.display(),
                        exceptions
                            .iter()
                            .map(|p| format!("<file>{}</file>", p.display()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))?;

                    let mut exclude = exceptions.to_owned();

                    // If we don't exclude the lock, it will trigger a permissions error
                    // when we attempt to remove it, failing the entire build
                    exclude.push(LOCK_FILE.into());

                    fs::remove_dir_all_except(dir, exclude)?;
                }
                BuildInstruction::RemoveDir(dir) => {
                    let dir = make_absolute(dir);

                    self.render_checkpoint(format!(
                        "{prefix} Removing directory <path>{}</path>",
                        dir.display()
                    ))?;

                    fs::remove_dir_all(dir)?;
                }
                BuildInstruction::RemoveFile(file) => {
                    let file = make_absolute(file);

                    self.render_checkpoint(format!(
                        "{prefix} Removing file <path>{}</path>",
                        file.display()
                    ))?;

                    fs::remove_file(file)?;
                }
                BuildInstruction::RequestScript(url) => {
                    let url = self.options.config.rewrite_url(url);
                    let filename = extract_file_name_from_url(&url);
                    let download_file = self.options.temp_dir.join(&filename);

                    self.render_checkpoint(format!("{prefix} Requesting script <url>{url}</url>"))?;

                    net::download_from_url_with_options(
                        &url,
                        &download_file,
                        DownloadOptions::new(
                            self.options
                                .http_client
                                .create_downloader_with_headers(build.http_headers.clone()),
                        ),
                    )
                    .await?;

                    fs::rename(download_file, self.options.install_dir.join(filename))?;
                }
                BuildInstruction::RunCommand(cmd) => {
                    let exe = if cmd.builder {
                        builder_exes.get(&cmd.exe).cloned().ok_or_else(|| {
                            ProtoBuildError::MissingBuilder {
                                id: Id::raw(&cmd.exe),
                            }
                        })?
                    } else {
                        PathBuf::from(&cmd.exe)
                    };

                    self.render_checkpoint(format!(
                        "{prefix} Running command <shell>{} {}</shell>",
                        fs::file_name(&exe),
                        shell_words::join(&cmd.args)
                    ))?;

                    self.exec_command(
                        Command::new(exe)
                            .args(&cmd.args)
                            .envs(&cmd.env)
                            .current_dir(
                                cmd.cwd
                                    .as_deref()
                                    .map(make_absolute)
                                    .unwrap_or_else(|| self.options.install_dir.to_path_buf()),
                            ),
                        false,
                    )
                    .await?;
                }
                BuildInstruction::SetEnvVar(key, value) => {
                    self.render_checkpoint(format!(
                        "{prefix} Setting environment variable <property>{key}</property> to <symbol>{value}</symbol>",
                    ))?;

                    unsafe { env::set_var(key, value) };
                }
            };
        }

        Ok(())
    }
}
