pub use super::build_error::*;
use super::install::{InstallPhase, OnPhaseFn};
use crate::components::check_line::CheckLine;
use crate::proto::ProtoConsole;
use iocraft::element;
use miette::IntoDiagnostic;
use proto_pdk_api::{BuildRequirement, SystemDependency};
use schematic::color::apply_style_tags;
use semver::Version;
use starbase_console::ui::{Container, Section};
use starbase_styles::color;
use system_env::{find_command_on_path, SystemArch, SystemOS};
use tokio::process::Command;
use tracing::{debug, error, trace};
use version_spec::get_semver_regex;

// TODO
// - phases

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
    pub fn new<'b>(options: &'b InstallBuildOptions) -> StepManager<'b> {
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
}

async fn run_command(cmd: &str, args: &[&str]) -> miette::Result<String> {
    let cmd_line = format!("{cmd} {}", shell_words::join(args));

    trace!("Running command {}", color::shell(&cmd_line));

    let output = Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|error| ProtoBuildError::CommandFailed {
            command: cmd_line.clone(),
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
        color::shell(&cmd_line)
    );

    if !output.status.success() {
        return Err(ProtoBuildError::CommandNonZeroExit {
            command: cmd_line.clone(),
            code,
        }
        .into());
    }

    Ok(stdout)
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
    let output = run_command(cmd, &[version_arg]).await?;

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

                let actual_value = run_command("git", &["config", "--get", config_key]).await?;

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

                    let result = run_command("xcode-select", &["--version"]).await;

                    if result.is_err() || result.is_ok_and(|out| out.is_empty()) {
                        step.render_check(
                            format!("Xcode command line tools are NOT installed, install them with <shell>xcode-select --install</shell>"),
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
