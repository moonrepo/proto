use super::build_error::*;
use super::install::{InstallPhase, OnPhaseFn};
use crate::config::ProtoBuildConfig;
use crate::env::{ProtoConsole, ProtoEnvironment};
use crate::helpers::extract_filename_from_url;
use crate::utils::process::{self, ProcessResult};
use crate::utils::{archive, git};
use iocraft::prelude::{FlexDirection, View, element};
use miette::IntoDiagnostic;
use proto_pdk_api::{
    BuildInstruction, BuildInstructionsOutput, BuildRequirement, GitSource, SourceLocation,
};
use rustc_hash::FxHashMap;
use schematic::color::{apply_style_tags, remove_style_tags};
use semver::{Version, VersionReq};
use starbase_console::ui::{
    Confirm, Container, Entry, ListCheck, ListItem, Section, Select, SelectOption, Style,
    StyledText,
};
use starbase_utils::fs::LOCK_FILE;
use starbase_utils::{env::is_ci, fs, net};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use system_env::{
    DependencyConfig, DependencyName, System, SystemPackageManager, find_command_on_path,
    is_command_on_path,
};
use tokio::process::Command;
use tokio::sync::{Mutex, OwnedMutexGuard};
use tracing::{debug, error};
use version_spec::{VersionSpec, get_semver_regex};
use warpgate::HttpClient;

static BUILD_LOCKS: OnceLock<scc::HashMap<String, Arc<Mutex<()>>>> = OnceLock::new();

pub struct BuilderOptions<'a> {
    pub config: &'a ProtoBuildConfig,
    pub console: &'a ProtoConsole,
    pub http_client: &'a HttpClient,
    pub install_dir: &'a Path,
    pub on_phase_change: Option<OnPhaseFn>,
    pub skip_prompts: bool,
    pub skip_ui: bool,
    pub system: System,
    pub temp_dir: &'a Path,
    pub version: VersionSpec,
}

enum BuilderStepOperation {
    Checkpoint(String),
    Command(Arc<ProcessResult>),
}

struct BuilderStep {
    title: String,
    ops: Vec<BuilderStepOperation>,
}

pub struct Builder<'a> {
    pub options: BuilderOptions<'a>,
    errors: u8,
    steps: Vec<BuilderStep>,
}

impl Builder<'_> {
    pub fn new(options: BuilderOptions<'_>) -> Builder<'_> {
        Builder {
            errors: 0,
            options,
            steps: vec![],
        }
    }

    pub fn get_system(&self) -> &System {
        &self.options.system
    }

    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }

    pub fn render_header(&mut self, title: impl AsRef<str>) -> miette::Result<()> {
        let title = title.as_ref();

        self.errors = 0;
        self.steps.push(BuilderStep {
            title: title.to_owned(),
            ops: vec![],
        });

        if self.options.skip_ui {
            debug!("{title}");
        } else {
            let console = &self.options.console;
            console.out.write_newline()?;
            console.render(element! {
                Container {
                    Section(title)
                }
            })?;
        }

        Ok(())
    }

    pub fn render_check(&mut self, message: impl AsRef<str>, passed: bool) -> miette::Result<()> {
        let message = message.as_ref();

        if self.options.skip_ui {
            let message = apply_style_tags(message);

            if passed {
                debug!("{message}");
            } else {
                error!("{message}");
            }
        } else {
            self.options.console.render(element! {
                ListCheck(checked: passed) {
                    StyledText(content: message)
                }
            })?;
        }

        if !passed {
            self.errors += 1;
        }

        Ok(())
    }

    pub fn render_checkpoint(&mut self, message: impl AsRef<str>) -> miette::Result<()> {
        let message = message.as_ref();

        if let Some(step) = &mut self.steps.last_mut() {
            step.ops
                .push(BuilderStepOperation::Checkpoint(message.to_owned()));
        }

        if self.options.skip_ui {
            debug!("{}", apply_style_tags(message));
        } else {
            self.options.console.render(element! {
                ListItem(bullet: "❯".to_owned()) {
                    StyledText(content: message)
                }
            })?;
        }

        Ok(())
    }

    pub async fn prompt_continue(&self, label: &str) -> miette::Result<()> {
        if self.options.skip_prompts || self.options.skip_ui {
            return Ok(());
        }

        let mut confirmed = false;

        self.options
            .console
            .render_interactive(element! {
                Confirm(label, on_confirm: &mut confirmed)
            })
            .await?;

        if !confirmed {
            return Err(ProtoBuildError::Cancelled.into());
        }

        Ok(())
    }

    pub async fn prompt_select(
        &self,
        label: &str,
        options: Vec<SelectOption>,
        default_index: usize,
    ) -> miette::Result<usize> {
        let mut selected_index = default_index;

        if self.options.skip_prompts || self.options.skip_ui {
            return Ok(selected_index);
        }

        self.options
            .console
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
    ) -> miette::Result<Arc<ProcessResult>> {
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
    ) -> miette::Result<Arc<ProcessResult>> {
        self.handle_process_result(if self.options.skip_ui || piped {
            process::exec_command_with_privileges_piped(command, elevated_program).await?
        } else {
            process::exec_command_with_privileges(command, elevated_program).await?
        })
    }

    fn handle_process_result(
        &mut self,
        result: ProcessResult,
    ) -> miette::Result<Arc<ProcessResult>> {
        let result = Arc::new(result);

        if let Some(step) = &mut self.steps.last_mut() {
            step.ops.push(BuilderStepOperation::Command(result.clone()));
        }

        if result.exit_code > 0 {
            return Err(process::ProtoProcessError::FailedCommandNonZeroExit {
                command: result.command.clone(),
                code: result.exit_code,
                stderr: result.stderr.clone(),
            }
            .into());
        }

        Ok(result)
    }

    pub fn write_log_file(&self, log_path: PathBuf) -> miette::Result<()> {
        let mut output = vec![];

        for (i, step) in self.steps.iter().enumerate() {
            output.push(format!("# Step {}: {}", i + 1, step.title));
            output.push("".into());

            for op in &step.ops {
                match op {
                    BuilderStepOperation::Checkpoint(title) => {
                        output.push(format!("## {}", remove_style_tags(title)));
                    }
                    BuilderStepOperation::Command(result) => {
                        output.push(format!("### `{}`", result.command));
                        output.push("".into());

                        if let Some(cwd) = &result.working_dir {
                            output.push(format!("WORKING DIR: {}", cwd.display()));
                            output.push("".into());
                        }

                        output.push(format!("EXIT CODE: {}", result.exit_code));
                        output.push("".into());

                        output.push("STDERR:".into());

                        if result.stderr.is_empty() {
                            output.push("".into());
                        } else {
                            output.push("```".into());
                            output.push(result.stderr.trim().to_owned());
                            output.push("```".into());
                        }

                        output.push("STDOUT:".into());

                        if result.stdout.is_empty() {
                            output.push("".into());
                        } else {
                            output.push("```".into());
                            output.push(result.stdout.trim().to_owned());
                            output.push("```".into());
                        }
                    }
                };

                output.push("".into());
            }
        }

        fs::write_file(log_path, output.join("\n"))?;

        Ok(())
    }

    pub async fn acquire_lock(&self, pm: &SystemPackageManager) -> OwnedMutexGuard<()> {
        let locks = BUILD_LOCKS.get_or_init(scc::HashMap::default);
        let entry = locks.entry(pm.to_string()).or_default();

        entry.get().clone().lock_owned().await
    }
}

async fn checkout_git_repo(
    git: &GitSource,
    cwd: &Path,
    builder: &mut Builder<'_>,
) -> miette::Result<()> {
    if cwd.join(".git").exists() {
        builder.exec_command(&mut git::new_pull(cwd), false).await?;

        return Ok(());
    }

    fs::create_dir_all(cwd)?;

    builder
        .exec_command(&mut git::new_clone(git, cwd), false)
        .await?;

    if let Some(reference) = &git.reference {
        builder.render_checkpoint(format!("Checking out reference <hash>{}</hash>", reference))?;

        builder
            .exec_command(&mut git::new_checkout(reference, cwd), false)
            .await?;
    }

    Ok(())
}

// STEP 0

pub fn log_build_information(
    builder: &mut Builder,
    build: &BuildInstructionsOutput,
) -> miette::Result<()> {
    let system = &builder.options.system;

    if builder.options.skip_ui {
        debug!(
            os = ?system.os,
            arch = ?system.arch,
            pm = ?system.manager,
            "Gathering system information",
        );
    } else {
        builder.options.console.render(element! {
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
                        StyledText(content: builder.options.version.to_string(), style: Style::Hash)
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

// STEP 1

pub async fn install_system_dependencies(
    builder: &mut Builder<'_>,
    build: &BuildInstructionsOutput,
) -> miette::Result<()> {
    let Some(pm) = builder.options.system.manager else {
        return Ok(());
    };

    // Determine packages to install
    let pm_config = pm.get_config();
    let dep_configs = builder
        .get_system()
        .resolve_dependencies(&build.system_dependencies);

    // 1) Check if packages have already been installed
    let mut not_installed_packages = FxHashMap::from_iter(
        dep_configs
            .iter()
            .filter_map(|cfg| cfg.get_package_names_and_versions(&pm).ok())
            .flatten(),
    );

    for excluded in &builder.options.config.exclude_packages {
        not_installed_packages.remove(excluded);
    }

    if not_installed_packages.is_empty() {
        return Ok(());
    }

    builder.render_header("Installing system dependencies")?;

    builder.options.on_phase_change.as_ref().inspect(|func| {
        func(InstallPhase::InstallDeps);
    });

    if let Ok(Some(mut list_args)) = builder
        .get_system()
        .get_list_packages_command(!builder.options.skip_prompts)
    {
        let _lock = builder.acquire_lock(&pm).await;

        builder.render_checkpoint(format!("Checking <shell>{pm}</shell> installed packages"))?;

        let list_output = builder
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
                        VersionReq::parse(required_version),
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
            builder.render_check(
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
        builder.render_check(
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
    let mut default_index = select_options.len() - 1;

    if let Some(sudo) = elevated_command {
        select_options.push(SelectOption::new(format!(
            "Yes, with elevated privileges ({sudo})"
        )));

        // Always run with elevated in CI
        if is_ci() {
            default_index += 1;
        }
    }

    match builder
        .prompt_select("Install missing packages?", select_options, default_index)
        .await?
    {
        0 => {
            return Err(ProtoBuildError::Cancelled.into());
        }
        1 => {
            return Ok(());
        }
        2 => {
            elevated_command = None;
        }
        _ => {}
    }

    // 3) Update the current registry index
    if let Some(mut index_args) = builder
        .get_system()
        .get_update_index_command(!builder.options.skip_prompts)
        .into_diagnostic()?
    {
        let _lock = builder.acquire_lock(&pm).await;

        builder.render_checkpoint("Updating package manager index")?;

        builder
            .exec_command_with_privileges(
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
    if let Some(mut install_args) = builder
        .get_system()
        .get_install_packages_command(&dep_configs, !builder.options.skip_prompts)
        .into_diagnostic()?
    {
        let _lock = builder.acquire_lock(&pm).await;

        builder.render_checkpoint(format!("Installing <shell>{pm}</shell> packages",))?;

        builder
            .exec_command_with_privileges(
                Command::new(install_args.remove(0)).args(install_args),
                elevated_command,
                false,
            )
            .await?;
    }

    Ok(())
}

// STEP 2

async fn get_command_version(
    cmd: &str,
    version_arg: &str,
    builder: &mut Builder<'_>,
) -> miette::Result<Version> {
    let output = builder
        .exec_command(Command::new(cmd).arg(version_arg), true)
        .await?;

    // Remove leading ^ and trailing $
    let base_pattern = get_semver_regex().as_str();
    let pattern = regex::Regex::new(&base_pattern[1..(base_pattern.len() - 1)]).unwrap();

    let value = pattern
        .find(&output.stdout)
        .map(|res| res.as_str())
        .unwrap_or(&output.stdout);

    Ok(
        Version::parse(value).map_err(|error| ProtoBuildError::FailedVersionParse {
            value: value.to_owned(),
            error: Box::new(error),
        })?,
    )
}

pub async fn check_requirements(
    builder: &mut Builder<'_>,
    build: &BuildInstructionsOutput,
) -> miette::Result<()> {
    if build.requirements.is_empty() {
        return Ok(());
    }

    builder.render_header("Checking requirements")?;

    builder.options.on_phase_change.as_ref().inspect(|func| {
        func(InstallPhase::CheckRequirements);
    });

    for req in &build.requirements {
        match req {
            BuildRequirement::CommandExistsOnPath(cmd) => {
                debug!(cmd, "Checking if a command exists on PATH");

                if let Some(cmd_path) = find_command_on_path(cmd) {
                    builder.render_check(
                        format!(
                            "Command <shell>{cmd}</shell> exists on PATH: <path>{}</path>",
                            cmd_path.display()
                        ),
                        true,
                    )?;
                } else {
                    builder.render_check(
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
                    let version = get_command_version(
                        cmd,
                        version_arg.as_deref().unwrap_or("--version"),
                        builder,
                    )
                    .await?;

                    if version_req.matches(&version) {
                        builder.render_check(
                            format!("Command <shell>{cmd}</shell> meets the minimum required version of {version_req}"),
                            true,
                        )?;
                    } else {
                        builder.render_check(
                            format!("Command <shell>{cmd}</shell> does NOT meet the minimum required version of {version_req}, found {version}"),
                            false,
                        )?;
                    }
                } else {
                    builder.render_check(
                        format!("Command <shell>{cmd}</shell> does NOT exist on PATH, please install it and try again"),
                        false,
                    )?;
                }
            }
            BuildRequirement::ManualIntercept(url) => {
                builder.render_check(
                    format!("Please read the following documentation before proceeding: <url>{url}</url>"),
                    true,
                )?;

                builder.prompt_continue("Continue install?").await?;
            }
            BuildRequirement::GitConfigSetting(config_key, expected_value) => {
                debug!(
                    config_key,
                    expected_value, "Checking if a Git config setting has the expected value"
                );

                let result = builder
                    .exec_command(
                        Command::new("git").args(["config", "--get", config_key]),
                        true,
                    )
                    .await?;
                let actual_value = &result.stdout;

                if actual_value == expected_value {
                    builder.render_check(
                        format!("Git config <property>{config_key}</property> matches the required value of <symbol>{expected_value}</symbol>"),
                        true,
                    )?;
                } else {
                    builder.render_check(
                        format!("Git config <property>{config_key}</property> does NOT match the required value or <symbol>{expected_value}</symbol>, found {actual_value}"),
                        false,
                    )?;
                }
            }
            BuildRequirement::GitVersion(version_req) => {
                debug!("Checking if Git meets the required version of {version_req}");

                let version = get_command_version("git", "--version", builder).await?;

                if version_req.matches(&version) {
                    builder.render_check(
                        format!("Git meets the minimum required version of {version_req}"),
                        true,
                    )?;
                } else {
                    builder.render_check(
                        format!("Git does NOT meet the minimum required version of {version_req}, found {version}"),
                        false,
                    )?;
                }
            }
            BuildRequirement::XcodeCommandLineTools => {
                if builder.get_system().os.is_mac() {
                    debug!("Checking if Xcode command line tools are installed");

                    let result = builder
                        .exec_command(Command::new("xcode-select").arg("--version"), true)
                        .await;

                    if result.is_err() || result.is_ok_and(|out| out.stdout.is_empty()) {
                        builder.render_check(
                            "Xcode command line tools are NOT installed, install them with <shell>xcode-select --install</shell>",
                            false,
                        )?;
                    } else {
                        builder.render_check("Xcode command line tools are installed", true)?;
                    }
                }
            }
            BuildRequirement::WindowsDeveloperMode => {
                if builder.get_system().os.is_windows() {
                    debug!("Checking if Windows developer mode is enabled");

                    // Is this possible from the command line?
                }
            }
        };
    }

    if builder.has_errors() {
        return Err(ProtoBuildError::RequirementsNotMet.into());
    }

    Ok(())
}

// STEP 3

pub async fn download_sources(
    builder: &mut Builder<'_>,
    build: &BuildInstructionsOutput,
) -> miette::Result<()> {
    // Ensure the install directory is empty, otherwise Git will fail and
    // we also want to avoid colliding/stale artifacts. This should also
    // run if there's no source, as it's required for instructions!
    fs::remove_dir_all(builder.options.install_dir)?;
    fs::create_dir_all(builder.options.install_dir)?;

    let Some(source) = &build.source else {
        return Ok(());
    };

    builder.render_header("Acquiring source files")?;

    match source {
        SourceLocation::Archive(archive) => {
            if archive::should_unpack(archive, builder.options.install_dir)? {
                let filename = extract_filename_from_url(&archive.url)?;

                // Download
                builder.options.on_phase_change.as_ref().inspect(|func| {
                    func(InstallPhase::Download {
                        url: archive.url.clone(),
                        file: filename.clone(),
                    });
                });

                builder.render_checkpoint(format!(
                    "Downloading archive from <url>{}</url>",
                    archive.url
                ))?;

                let download_file = archive::download(
                    archive,
                    builder.options.temp_dir,
                    builder.options.http_client.to_inner(),
                )
                .await?;

                // Unpack
                builder.options.on_phase_change.as_ref().inspect(|func| {
                    func(InstallPhase::Unpack {
                        file: filename.clone(),
                    });
                });

                builder.render_checkpoint(format!(
                    "Unpacking archive to <path>{}</path>",
                    builder.options.install_dir.display()
                ))?;

                archive::unpack(archive, builder.options.install_dir, &download_file)?;
            }
        }
        SourceLocation::Git(git) => {
            builder.options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::CloneRepository {
                    url: git.url.clone(),
                });
            });

            builder.render_checkpoint(format!("Cloning repository <url>{}</url>", git.url))?;

            checkout_git_repo(git, builder.options.install_dir, builder).await?;
        }
    };

    Ok(())
}

// STEP 4

pub async fn execute_instructions(
    builder: &mut Builder<'_>,
    build: &BuildInstructionsOutput,
    proto: &ProtoEnvironment,
) -> miette::Result<()> {
    if build.instructions.is_empty() {
        return Ok(());
    }

    builder.render_header("Executing build instructions")?;

    builder.options.on_phase_change.as_ref().inspect(|func| {
        func(InstallPhase::ExecuteInstructions);
    });

    let make_absolute = |path: &Path| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            builder.options.install_dir.join(path)
        }
    };

    let total = build.instructions.len();
    let mut builder_exes = FxHashMap::default();

    for (index, instruction) in build.instructions.iter().enumerate() {
        debug!("Executing build instruction {} of {total}", index + 1);

        let prefix = format!("<mutedlight>[{}/{total}]</mutedlight>", index + 1);

        match instruction {
            BuildInstruction::InstallBuilder(item) => {
                builder.render_checkpoint(format!(
                    "{prefix} Installing <id>{}</id> builder (<url>{}<url>)",
                    item.id, item.git.url
                ))?;

                let builder_dir = proto.store.builders_dir.join(&item.id);

                checkout_git_repo(&item.git, &builder_dir, builder).await?;

                let main_exe_name = String::new();
                let mut exes = FxHashMap::default();
                exes.extend(&item.exes);
                exes.insert(&main_exe_name, &item.exe);

                for (exe_name, exe_rel_path) in exes {
                    let exe_abs_path = builder_dir.join(exe_rel_path);

                    if !exe_abs_path.exists() {
                        return Err(ProtoBuildError::MissingBuilderExe {
                            exe: exe_abs_path,
                            id: item.id.clone(),
                        }
                        .into());
                    }

                    fs::update_perms(&exe_abs_path, None)?;

                    builder_exes.insert(
                        if exe_name.is_empty() {
                            item.id.clone()
                        } else {
                            format!("{}:{exe_name}", item.id)
                        },
                        exe_abs_path,
                    );
                }
            }
            BuildInstruction::MakeExecutable(file) => {
                let file = make_absolute(file);

                builder.render_checkpoint(format!(
                    "{prefix} Making file <path>{}</path> executable",
                    file.display()
                ))?;

                fs::update_perms(file, None)?;
            }
            BuildInstruction::MoveFile(from, to) => {
                let from = make_absolute(from);
                let to = make_absolute(to);

                builder.render_checkpoint(format!(
                    "{prefix} Moving <path>{}</path> to <path>{}</path>",
                    from.display(),
                    to.display(),
                ))?;

                fs::rename(from, to)?;
            }
            BuildInstruction::RemoveAllExcept(exceptions) => {
                let dir = builder.options.install_dir;

                builder.render_checkpoint(format!(
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

                builder.render_checkpoint(format!(
                    "{prefix} Removing directory <path>{}</path>",
                    dir.display()
                ))?;

                fs::remove_dir_all(dir)?;
            }
            BuildInstruction::RemoveFile(file) => {
                let file = make_absolute(file);

                builder.render_checkpoint(format!(
                    "{prefix} Removing file <path>{}</path>",
                    file.display()
                ))?;

                fs::remove_file(file)?;
            }
            BuildInstruction::RequestScript(url) => {
                let filename = extract_filename_from_url(url)?;
                let download_file = builder.options.temp_dir.join(&filename);

                builder
                    .render_checkpoint(format!("{prefix} Requesting script <url>{url}</url>"))?;

                net::download_from_url_with_client(
                    url,
                    &download_file,
                    builder.options.http_client.to_inner(),
                )
                .await?;

                fs::rename(download_file, builder.options.install_dir.join(filename))?;
            }
            BuildInstruction::RunCommand(cmd) => {
                let exe = if cmd.builder {
                    builder_exes.get(&cmd.bin).cloned().ok_or_else(|| {
                        ProtoBuildError::MissingBuilder {
                            id: cmd.bin.clone(),
                        }
                    })?
                } else {
                    PathBuf::from(&cmd.bin)
                };

                builder.render_checkpoint(format!(
                    "{prefix} Running command <shell>{} {}</shell>",
                    exe.file_name().unwrap().to_str().unwrap(),
                    shell_words::join(&cmd.args)
                ))?;

                builder
                    .exec_command(
                        Command::new(exe)
                            .args(&cmd.args)
                            .envs(&cmd.env)
                            .current_dir(
                                cmd.cwd
                                    .as_deref()
                                    .map(make_absolute)
                                    .unwrap_or_else(|| builder.options.install_dir.to_path_buf()),
                            ),
                        false,
                    )
                    .await?;
            }
            BuildInstruction::SetEnvVar(key, value) => {
                builder.render_checkpoint(format!(
                    "{prefix} Setting environment variable <property>{key}</property> to <symbol>{value}</symbol>",
                ))?;

                unsafe { env::set_var(key, value) };
            }
        };
    }

    Ok(())
}
