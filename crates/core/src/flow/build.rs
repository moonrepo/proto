use crate::proto::ProtoConsole;
use iocraft::element;
use miette::IntoDiagnostic;
use proto_pdk_api::{BuildRequirement, SystemDependency};
use schematic::color::apply_style_tags;
use semver::Version;
use starbase_console::ui::{Container, ListItem, Section, StyledText};
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
    pub skip_prompts: bool,
}

struct StepManager<'a> {
    errors: Vec<String>,
    options: &'a InstallBuildOptions,
}

impl StepManager<'_> {
    pub fn new<'b>(options: &'b InstallBuildOptions) -> StepManager<'b> {
        StepManager {
            errors: vec![],
            options,
        }
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
                ListItem(
                    bullet: if passed {
                        "✔".to_owned()
                    } else {
                        "✘".to_owned()
                    }
                ) {
                    StyledText(content: message)
                }
            })?;

            if !passed {
                self.errors.push(apply_style_tags(message));
            }
        } else {
            let message = apply_style_tags(message);

            if passed {
                debug!("{message}");
            } else {
                error!("{message}");
                self.errors.push(message);
            }
        }

        Ok(())
    }
}

async fn run_command(cmd: &str, args: &[&str]) -> miette::Result<String> {
    let cmd_line = color::shell(format!("{cmd} {}", args.join(" ")));

    trace!("Running command {cmd_line}");

    let output = Command::new(cmd)
        .args(args)
        .output()
        .await
        .into_diagnostic()?;

    if !output.status.success() {
        panic!();
    }

    let stdout = String::from_utf8(output.stdout).into_diagnostic()?;

    trace!(stdout, "Ran command {cmd_line}");

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

    Ok(())
}

// STEP 2

async fn get_command_version(cmd: &str, version_arg: &str) -> miette::Result<Version> {
    let output = run_command(cmd, &[version_arg]).await?;

    // Remove leading ^ and trailing $
    let base_pattern = get_semver_regex().as_str();
    let pattern = regex::Regex::new(&base_pattern[1..(base_pattern.len() - 1)]).unwrap();

    Ok(Version::parse(
        pattern
            .find(&output)
            .map(|res| res.as_str())
            .unwrap_or(&output),
    )
    .into_diagnostic()?)
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

    for req in reqs {
        match req {
            BuildRequirement::CommandExistsOnPath(cmd) => {
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
                    // Is this possible from the command line?
                }
            }
        };
    }

    Ok(())
}
