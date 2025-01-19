pub use super::build_error::*;
use super::install::{InstallPhase, OnPhaseFn};
use crate::components::check_line::CheckLine;
use crate::proto::ProtoConsole;
use iocraft::element;
use miette::IntoDiagnostic;
use proto_pdk_api::{BuildInstruction, BuildRequirement, SystemDependency};
use schematic::color::apply_style_tags;
use semver::Version;
use starbase_console::ui::{Container, ListItem, Section, StyledText};
use starbase_styles::color;
use starbase_utils::fs;
use std::path::Path;
use std::process::Stdio;
use system_env::{find_command_on_path, SystemArch, SystemOS};
use tokio::process::Command;
use tracing::{debug, error, trace};
use version_spec::get_semver_regex;

#[derive(Default)]
pub struct InstallBuildOptions {
    pub console: Option<ProtoConsole>,
    pub host_arch: SystemArch,
    pub host_os: SystemOS,
    pub on_phase_change: Option<OnPhaseFn>,
    pub skip_prompts: bool,
}

struct StepManager<'a> {
    errors: u8,
    options: &'a InstallBuildOptions,
}

impl StepManager<'_> {
    pub fn new(options: &InstallBuildOptions) -> StepManager<'_> {
        StepManager { errors: 0, options }
    }

    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }

    pub fn render_header(&self, title: impl AsRef<str>) -> miette::Result<()> {
        let title = title.as_ref();

        if let Some(console) = &self.options.console {
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
                CheckLine(passed, message)
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
                ListItem {
                    StyledText(content: message)
                }
            })?;
        } else {
            debug!("{}", apply_style_tags(message));
        }

        Ok(())
    }
}

async fn exec_command(command: &mut Command) -> miette::Result<String> {
    let command_line = {
        let inner = command.as_std();
        let args = inner
            .get_args()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>();

        format!("{:?} {}", inner.get_program(), shell_words::join(args))
    };

    trace!("Running command {}", color::shell(&command_line));

    let child = command
        .spawn()
        .map_err(|error| ProtoBuildError::CommandFailed {
            command: command_line.clone(),
            error: Box::new(error),
        })?;

    let output =
        child
            .wait_with_output()
            .await
            .map_err(|error| ProtoBuildError::CommandFailed {
                command: command_line.clone(),
                error: Box::new(error),
            })?;

    let stderr = String::from_utf8(output.stderr).into_diagnostic()?;
    let stdout = String::from_utf8(output.stdout).into_diagnostic()?;
    let code = output.status.code().unwrap_or(-1);

    trace!(
        code,
        stderr,
        stdout,
        "Ran command {}",
        color::shell(&command_line)
    );

    if !output.status.success() {
        return Err(ProtoBuildError::CommandNonZeroExit {
            command: command_line.clone(),
            code,
        }
        .into());
    }

    Ok(stdout)
}

async fn exec_command_piped(command: &mut Command) -> miette::Result<String> {
    exec_command(command.stderr(Stdio::piped()).stdout(Stdio::piped())).await
}

// STEP 1

pub async fn install_system_dependencies(
    deps: &[SystemDependency],
    options: &InstallBuildOptions,
) -> miette::Result<()> {
    if deps.is_empty() {
        return Ok(());
    }

    let step = StepManager::new(options);

    step.render_header("Installing system dependencies")?;

    options.on_phase_change.as_ref().inspect(|func| {
        func(InstallPhase::InstallDeps);
    });

    Ok(())
}

// STEP 2

async fn get_command_version(cmd: &str, version_arg: &str) -> miette::Result<Version> {
    let output = exec_command_piped(Command::new(cmd).arg(version_arg)).await?;

    // Remove leading ^ and trailing $
    let base_pattern = get_semver_regex().as_str();
    let pattern = regex::Regex::new(&base_pattern[1..(base_pattern.len() - 1)]).unwrap();

    let value = pattern
        .find(&output)
        .map(|res| res.as_str())
        .unwrap_or(&output);

    Ok(
        Version::parse(value).map_err(|error| ProtoBuildError::VersionParseFailed {
            value: value.to_owned(),
            error: Box::new(error),
        })?,
    )
}

pub async fn check_requirements(
    reqs: &[BuildRequirement],
    options: &InstallBuildOptions,
) -> miette::Result<()> {
    if reqs.is_empty() {
        return Ok(());
    }

    let mut step = StepManager::new(options);

    step.render_header("Checking requirements")?;

    options.on_phase_change.as_ref().inspect(|func| {
        func(InstallPhase::CheckRequirements);
    });

    for req in reqs {
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

                let version =
                    get_command_version(cmd, version_arg.as_deref().unwrap_or("--version")).await?;

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
            }
            BuildRequirement::ManualIntercept(url) => {
                step.render_check(
                    format!("Please read the following documentation before proceeding: <url>{url}</url>"),
                    true,
                )?;
            }
            BuildRequirement::GitConfigSetting(config_key, expected_value) => {
                debug!(
                    config_key,
                    expected_value, "Checking if a Git config setting has the expected value"
                );

                let actual_value =
                    exec_command_piped(Command::new("git").args(["config", "--get", config_key]))
                        .await?;

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
                if options.host_os.is_mac() {
                    debug!("Checking if Xcode command line tools are installed");

                    let result =
                        exec_command_piped(Command::new("xcode-select").arg("--version")).await;

                    if result.is_err() || result.is_ok_and(|out| out.is_empty()) {
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
                if options.host_os.is_windows() {
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

// STEP 4

pub async fn execute_instructions(
    instructions: &[BuildInstruction],
    options: &InstallBuildOptions,
    install_dir: &Path,
) -> miette::Result<()> {
    if instructions.is_empty() {
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
            install_dir.join(path)
        }
    };

    let total = instructions.len();

    for (index, instruction) in instructions.iter().enumerate() {
        debug!("Executing build instruction {} of {total}", index + 1);

        match instruction {
            BuildInstruction::MakeExecutable(file) => {
                step.render_checkpoint(format!(
                    "Making file <file>{}</file> executable",
                    file.display()
                ))?;

                fs::update_perms(make_absolute(file), None)?;
            }
            BuildInstruction::MoveFile(from, to) => {
                step.render_checkpoint(format!(
                    "Moving <file>{}</file> to <file>{}</file>",
                    from.display(),
                    to.display(),
                ))?;

                fs::rename(make_absolute(from), make_absolute(to))?;
            }
            BuildInstruction::RemoveDir(dir) => {
                step.render_checkpoint(format!(
                    "Removing directory <file>{}</file>",
                    dir.display()
                ))?;

                fs::remove_dir_all(make_absolute(dir))?;
            }
            BuildInstruction::RemoveFile(file) => {
                step.render_checkpoint(format!("Removing file <file>{}</file>", file.display()))?;

                fs::remove_file(make_absolute(file))?;
            }
            BuildInstruction::RequestScript(_) => {
                unimplemented!(); // TODO
            }
            BuildInstruction::RunCommand(cmd) => {
                step.render_checkpoint(format!(
                    "Running command <shell>{} {}</shell>",
                    cmd.bin,
                    shell_words::join(&cmd.args)
                ))?;

                exec_command(
                    Command::new(&cmd.bin)
                        .args(&cmd.args)
                        .envs(&cmd.env)
                        .current_dir(
                            cmd.cwd
                                .as_deref()
                                .map(make_absolute)
                                .unwrap_or_else(|| install_dir.to_path_buf()),
                        ),
                )
                .await?;
            }
        };
    }

    Ok(())
}
