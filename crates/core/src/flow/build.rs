use super::build_error::*;
use super::install::{InstallPhase, OnPhaseFn};
use crate::config::ProtoBuildConfig;
use crate::env::{ProtoConsole, ProtoEnvironment};
use crate::helpers::extract_filename_from_url;
use crate::utils::process::*;
use iocraft::prelude::{element, FlexDirection, View};
use miette::IntoDiagnostic;
use proto_pdk_api::{
    BuildInstruction, BuildInstructionsOutput, BuildRequirement, GitSource, SourceLocation,
};
use rustc_hash::FxHashMap;
use schematic::color::apply_style_tags;
use semver::{Version, VersionReq};
use starbase_archive::Archiver;
use starbase_console::ui::{
    Confirm, Container, Entry, ListCheck, ListItem, Section, Select, SelectOption, Style,
    StyledText,
};
use starbase_utils::fs::LOCK_FILE;
use starbase_utils::{env::is_ci, fs, net};
use std::env;
use std::path::{Path, PathBuf};
use system_env::{
    find_command_on_path, is_command_on_path, DependencyConfig, DependencyName, System,
};
use tokio::process::Command;
use tracing::{debug, error};
use version_spec::{get_semver_regex, VersionSpec};
use warpgate::HttpClient;

pub struct InstallBuildOptions<'a> {
    pub config: &'a ProtoBuildConfig,
    pub console: Option<&'a ProtoConsole>,
    pub http_client: &'a HttpClient,
    pub install_dir: &'a Path,
    pub on_phase_change: Option<OnPhaseFn>,
    pub skip_prompts: bool,
    pub system: System,
    pub temp_dir: &'a Path,
    pub version: VersionSpec,
}

struct StepManager<'a> {
    errors: u8,
    options: &'a InstallBuildOptions<'a>,
}

impl StepManager<'_> {
    pub fn new<'a>(options: &'a InstallBuildOptions<'a>) -> StepManager<'a> {
        StepManager { errors: 0, options }
    }

    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }

    pub fn render_header(&self, title: impl AsRef<str>) -> miette::Result<()> {
        let title = title.as_ref();

        if let Some(console) = &self.options.console {
            console.out.write_newline()?;
            console.render(element! {
                Container {
                    Section(title)
                }
            })?;
        } else {
            debug!("{title}");
        }

        Ok(())
    }

    pub fn render_check(&mut self, message: impl AsRef<str>, passed: bool) -> miette::Result<()> {
        let message = message.as_ref();

        if let Some(console) = &self.options.console {
            console.render(element! {
                ListCheck(checked: passed) {
                    StyledText(content: message)
                }
            })?;
        } else {
            let message = apply_style_tags(message);

            if passed {
                debug!("{message}");
            } else {
                error!("{message}");
            }
        }

        if !passed {
            self.errors += 1;
        }

        Ok(())
    }

    pub fn render_checkpoint(&self, message: impl AsRef<str>) -> miette::Result<()> {
        let message = message.as_ref();

        if let Some(console) = &self.options.console {
            console.render(element! {
                ListItem(bullet: "❯".to_owned()) {
                    StyledText(content: message)
                }
            })?;
        } else {
            debug!("{}", apply_style_tags(message));
        }

        Ok(())
    }

    pub async fn prompt_continue(&self, label: &str) -> miette::Result<()> {
        if self.options.skip_prompts {
            return Ok(());
        }

        if let Some(console) = &self.options.console {
            let mut confirmed = false;

            console
                .render_interactive(element! {
                    Confirm(label, on_confirm: &mut confirmed)
                })
                .await?;

            if !confirmed {
                return Err(ProtoBuildError::Cancelled.into());
            }
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

        if self.options.skip_prompts {
            return Ok(selected_index);
        }

        if let Some(console) = &self.options.console {
            console
                .render_interactive(element! {
                    Select(label, options, on_index: &mut selected_index)
                })
                .await?;
        }

        Ok(selected_index)
    }
}

async fn checkout_git_repo(
    git: &GitSource,
    cwd: &Path,
    step: &StepManager<'_>,
) -> miette::Result<()> {
    if cwd.join(".git").exists() {
        exec_command(
            Command::new("git")
                .args(["pull", "--ff", "--prune"])
                .current_dir(cwd),
        )
        .await?;

        return Ok(());
    }

    fs::create_dir_all(cwd)?;

    exec_command(
        Command::new("git")
            .args(if git.submodules {
                vec!["clone", "--recurse-submodules"]
            } else {
                vec!["clone"]
            })
            .args(["--depth", "1"])
            .arg(&git.url)
            .arg(".")
            .current_dir(cwd),
    )
    .await?;

    if let Some(reference) = &git.reference {
        step.render_checkpoint(format!("Checking out reference <hash>{}</hash>", reference))?;

        exec_command(
            Command::new("git")
                .arg("checkout")
                .arg(reference)
                .current_dir(cwd),
        )
        .await?;
    }

    Ok(())
}

// STEP 0

pub fn log_build_information(
    build: &BuildInstructionsOutput,
    options: &InstallBuildOptions<'_>,
) -> miette::Result<()> {
    let system = &options.system;

    if let Some(console) = &options.console {
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
                        StyledText(content: options.version.to_string(), style: Style::Hash)
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
    } else {
        debug!(
            os = ?system.os,
            arch = ?system.arch,
            pm = ?system.manager,
            "Gathering system information",
        );
    }

    Ok(())
}

// STEP 1

pub async fn install_system_dependencies(
    build: &BuildInstructionsOutput,
    options: &InstallBuildOptions<'_>,
) -> miette::Result<()> {
    let system = &options.system;

    let Some(pm) = system.manager else {
        return Ok(());
    };

    // Determine packages to install
    let pm_config = pm.get_config();
    let dep_configs = system.resolve_dependencies(&build.system_dependencies);

    // 1) Check if packages have already been installed
    let mut not_installed_packages = FxHashMap::from_iter(
        dep_configs
            .iter()
            .filter_map(|cfg| cfg.get_package_names_and_versions(&pm).ok())
            .flatten(),
    );

    for excluded in &options.config.exclude_packages {
        not_installed_packages.remove(excluded);
    }

    if not_installed_packages.is_empty() {
        return Ok(());
    }

    let mut step = StepManager::new(options);
    step.render_header("Installing system dependencies")?;

    options.on_phase_change.as_ref().inspect(|func| {
        func(InstallPhase::InstallDeps);
    });

    if let Some(mut list_args) = system
        .get_list_packages_command(!options.skip_prompts)
        .into_diagnostic()?
    {
        step.render_checkpoint(format!("Checking <shell>{pm}</shell> installed packages"))?;

        let list_output =
            exec_command_piped(Command::new(list_args.remove(0)).args(list_args)).await?;
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
            step.render_check(
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
        step.render_check(
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

    match step
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
    if let Some(mut index_args) = system
        .get_update_index_command(!options.skip_prompts)
        .into_diagnostic()?
    {
        step.render_checkpoint("Updating package manager index")?;

        exec_command_with_privileges(
            Command::new(index_args.remove(0)).args(index_args),
            elevated_command,
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
    if let Some(mut install_args) = system
        .get_install_packages_command(&dep_configs, !options.skip_prompts)
        .into_diagnostic()?
    {
        step.render_checkpoint(format!("Installing <shell>{pm}</shell> packages",))?;

        exec_command_with_privileges(
            Command::new(install_args.remove(0)).args(install_args),
            elevated_command,
        )
        .await?;
    }

    Ok(())
}

// STEP 2

async fn get_command_version(cmd: &str, version_arg: &str) -> miette::Result<Version> {
    let output = exec_command_piped(Command::new(cmd).arg(version_arg))
        .await?
        .stdout;

    // Remove leading ^ and trailing $
    let base_pattern = get_semver_regex().as_str();
    let pattern = regex::Regex::new(&base_pattern[1..(base_pattern.len() - 1)]).unwrap();

    let value = pattern
        .find(&output)
        .map(|res| res.as_str())
        .unwrap_or(&output);

    Ok(
        Version::parse(value).map_err(|error| ProtoBuildError::FailedVersionParse {
            value: value.to_owned(),
            error: Box::new(error),
        })?,
    )
}

pub async fn check_requirements(
    build: &BuildInstructionsOutput,
    options: &InstallBuildOptions<'_>,
) -> miette::Result<()> {
    if build.requirements.is_empty() {
        return Ok(());
    }

    let mut step = StepManager::new(options);

    step.render_header("Checking requirements")?;

    options.on_phase_change.as_ref().inspect(|func| {
        func(InstallPhase::CheckRequirements);
    });

    for req in &build.requirements {
        match req {
            BuildRequirement::CommandExistsOnPath(cmd) => {
                debug!(cmd, "Checking if a command exists on PATH");

                if let Some(cmd_path) = find_command_on_path(cmd) {
                    step.render_check(
                        format!(
                            "Command <shell>{cmd}</shell> exists on PATH: <path>{}</path>",
                            cmd_path.display()
                        ),
                        true,
                    )?;
                } else {
                    step.render_check(
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
                    let version =
                        get_command_version(cmd, version_arg.as_deref().unwrap_or("--version"))
                            .await?;

                    if version_req.matches(&version) {
                        step.render_check(
                            format!("Command <shell>{cmd}</shell> meets the minimum required version of {version_req}"),
                            true,
                        )?;
                    } else {
                        step.render_check(
                            format!("Command <shell>{cmd}</shell> does NOT meet the minimum required version of {version_req}, found {version}"),
                            false,
                        )?;
                    }
                } else {
                    step.render_check(
                        format!("Command <shell>{cmd}</shell> does NOT exist on PATH, please install it and try again"),
                        false,
                    )?;
                }
            }
            BuildRequirement::ManualIntercept(url) => {
                step.render_check(
                    format!("Please read the following documentation before proceeding: <url>{url}</url>"),
                    true,
                )?;

                step.prompt_continue("Continue install?").await?;
            }
            BuildRequirement::GitConfigSetting(config_key, expected_value) => {
                debug!(
                    config_key,
                    expected_value, "Checking if a Git config setting has the expected value"
                );

                let actual_value =
                    exec_command_piped(Command::new("git").args(["config", "--get", config_key]))
                        .await?
                        .stdout;

                if &actual_value == expected_value {
                    step.render_check(
                        format!("Git config <property>{config_key}</property> matches the required value of <symbol>{expected_value}</symbol>"),
                        true,
                    )?;
                } else {
                    step.render_check(
                        format!("Git config <property>{config_key}</property> does NOT match the required value or <symbol>{expected_value}</symbol>, found {actual_value}"),
                        false,
                    )?;
                }
            }
            BuildRequirement::GitVersion(version_req) => {
                debug!("Checking if Git meets the required version of {version_req}");

                let version = get_command_version("git", "--version").await?;

                if version_req.matches(&version) {
                    step.render_check(
                        format!("Git meets the minimum required version of {version_req}"),
                        true,
                    )?;
                } else {
                    step.render_check(
                        format!("Git does NOT meet the minimum required version of {version_req}, found {version}"),
                        false,
                    )?;
                }
            }
            BuildRequirement::XcodeCommandLineTools => {
                if options.system.os.is_mac() {
                    debug!("Checking if Xcode command line tools are installed");

                    let result =
                        exec_command_piped(Command::new("xcode-select").arg("--version")).await;

                    if result.is_err() || result.is_ok_and(|out| out.stdout.is_empty()) {
                        step.render_check(
                            "Xcode command line tools are NOT installed, install them with <shell>xcode-select --install</shell>",
                            false,
                        )?;
                    } else {
                        step.render_check("Xcode command line tools are installed", true)?;
                    }
                }
            }
            BuildRequirement::WindowsDeveloperMode => {
                if options.system.os.is_windows() {
                    debug!("Checking if Windows developer mode is enabled");

                    // Is this possible from the command line?
                }
            }
        };
    }

    if step.has_errors() {
        return Err(ProtoBuildError::RequirementsNotMet.into());
    }

    Ok(())
}

// STEP 3

pub async fn download_sources(
    build: &BuildInstructionsOutput,
    options: &InstallBuildOptions<'_>,
) -> miette::Result<()> {
    // Ensure the install directory is empty, otherwise Git will fail and
    // we also want to avoid colliding/stale artifacts. This should also
    // run if there's no source, as it's required for instructions!
    fs::remove_dir_all(options.install_dir)?;
    fs::create_dir_all(options.install_dir)?;

    let Some(source) = &build.source else {
        return Ok(());
    };

    let step = StepManager::new(options);

    step.render_header("Acquiring source files")?;

    match source {
        SourceLocation::Archive(archive) => {
            let filename = extract_filename_from_url(&archive.url)?;
            let download_file = options.temp_dir.join(&filename);

            // Download
            options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::Download {
                    url: archive.url.clone(),
                    file: filename.clone(),
                });
            });

            step.render_checkpoint(format!(
                "Downloading archive from <url>{}</url>",
                archive.url
            ))?;

            net::download_from_url_with_client(
                &archive.url,
                &download_file,
                options.http_client.to_inner(),
            )
            .await?;

            // Unpack
            options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::Unpack {
                    file: filename.clone(),
                });
            });

            step.render_checkpoint(format!(
                "Unpacking archive to <path>{}</path>",
                options.install_dir.display()
            ))?;

            let mut archiver = Archiver::new(options.install_dir, &download_file);

            if let Some(prefix) = &archive.prefix {
                archiver.set_prefix(prefix);
            }

            archiver.unpack_from_ext()?;
        }
        SourceLocation::Git(git) => {
            options.on_phase_change.as_ref().inspect(|func| {
                func(InstallPhase::CloneRepository {
                    url: git.url.clone(),
                });
            });

            step.render_checkpoint(format!("Cloning repository <url>{}</url>", git.url))?;

            checkout_git_repo(git, options.install_dir, &step).await?;
        }
    };

    Ok(())
}

// STEP 4

pub async fn execute_instructions(
    build: &BuildInstructionsOutput,
    options: &InstallBuildOptions<'_>,
    proto: &ProtoEnvironment,
) -> miette::Result<()> {
    if build.instructions.is_empty() {
        return Ok(());
    }

    let step = StepManager::new(options);

    step.render_header("Executing build instructions")?;

    options.on_phase_change.as_ref().inspect(|func| {
        func(InstallPhase::ExecuteInstructions);
    });

    let make_absolute = |path: &Path| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            options.install_dir.join(path)
        }
    };

    let total = build.instructions.len();
    let mut builder_exes = FxHashMap::default();

    for (index, instruction) in build.instructions.iter().enumerate() {
        debug!("Executing build instruction {} of {total}", index + 1);

        let prefix = format!("<mutedlight>[{}/{total}]</mutedlight>", index + 1);

        match instruction {
            BuildInstruction::InstallBuilder(builder) => {
                step.render_checkpoint(format!(
                    "{prefix} Installing <id>{}</id> builder (<url>{}<url>)",
                    builder.id, builder.git.url
                ))?;

                let builder_dir = proto.store.builders_dir.join(&builder.id);

                checkout_git_repo(&builder.git, &builder_dir, &step).await?;

                let main_exe_name = String::new();
                let mut exes = FxHashMap::default();
                exes.extend(&builder.exes);
                exes.insert(&main_exe_name, &builder.exe);

                for (exe_name, exe_rel_path) in exes {
                    let exe_abs_path = builder_dir.join(exe_rel_path);

                    if !exe_abs_path.exists() {
                        return Err(ProtoBuildError::MissingBuilderExe {
                            exe: exe_abs_path,
                            id: builder.id.clone(),
                        }
                        .into());
                    }

                    fs::update_perms(&exe_abs_path, None)?;

                    builder_exes.insert(
                        if exe_name.is_empty() {
                            builder.id.clone()
                        } else {
                            format!("{}:{exe_name}", builder.id)
                        },
                        exe_abs_path,
                    );
                }
            }
            BuildInstruction::MakeExecutable(file) => {
                let file = make_absolute(file);

                step.render_checkpoint(format!(
                    "{prefix} Making file <path>{}</path> executable",
                    file.display()
                ))?;

                fs::update_perms(file, None)?;
            }
            BuildInstruction::MoveFile(from, to) => {
                let from = make_absolute(from);
                let to = make_absolute(to);

                step.render_checkpoint(format!(
                    "{prefix} Moving <path>{}</path> to <path>{}</path>",
                    from.display(),
                    to.display(),
                ))?;

                fs::rename(from, to)?;
            }
            BuildInstruction::RemoveAllExcept(exceptions) => {
                let dir = options.install_dir;

                step.render_checkpoint(format!(
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

                step.render_checkpoint(format!(
                    "{prefix} Removing directory <path>{}</path>",
                    dir.display()
                ))?;

                fs::remove_dir_all(dir)?;
            }
            BuildInstruction::RemoveFile(file) => {
                let file = make_absolute(file);

                step.render_checkpoint(format!(
                    "{prefix} Removing file <path>{}</path>",
                    file.display()
                ))?;

                fs::remove_file(file)?;
            }
            BuildInstruction::RequestScript(url) => {
                let filename = extract_filename_from_url(url)?;
                let download_file = options.temp_dir.join(&filename);

                step.render_checkpoint(format!("{prefix} Requesting script <url>{url}</url>"))?;

                net::download_from_url_with_client(
                    url,
                    &download_file,
                    options.http_client.to_inner(),
                )
                .await?;

                fs::rename(download_file, options.install_dir.join(filename))?;
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

                step.render_checkpoint(format!(
                    "{prefix} Running command <shell>{} {}</shell>",
                    exe.file_name().unwrap().to_str().unwrap(),
                    shell_words::join(&cmd.args)
                ))?;

                exec_command(
                    Command::new(exe)
                        .args(&cmd.args)
                        .envs(&cmd.env)
                        .current_dir(
                            cmd.cwd
                                .as_deref()
                                .map(make_absolute)
                                .unwrap_or_else(|| options.install_dir.to_path_buf()),
                        ),
                )
                .await?;
            }
            BuildInstruction::SetEnvVar(key, value) => {
                step.render_checkpoint(format!(
                    "{prefix} Setting environment variable <property>{key}</property> to <symbol>{value}</symbol>",
                ))?;

                env::set_var(key, value);
            }
        };
    }

    Ok(())
}
