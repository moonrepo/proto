use crate::deps::{DependencyConfig, SystemDependency};
use crate::env::*;
use crate::error::Error;
use crate::pm::*;
use crate::pm_vendor::*;

/// Represents the current system, including architecture, operating system,
/// and package manager information.
#[derive(Debug)]
pub struct System {
    /// Platform architecture.
    pub arch: SystemArch,

    /// Package manager.
    pub manager: SystemPackageManager,

    /// Operating system.
    pub os: SystemOS,
}

impl System {
    /// Create a new instance and detect system information.
    pub fn new() -> Result<Self, Error> {
        Ok(System::with_manager(SystemPackageManager::detect()?))
    }

    /// Create a new instance with the provided package manager.
    pub fn with_manager(manager: SystemPackageManager) -> Self {
        System {
            arch: SystemArch::from_env(),
            manager,
            os: SystemOS::from_env(),
        }
    }

    /// Return the command and arguments to "install a package" for the
    /// current package manager. Will replace `$` in an argument with the
    /// dependency name, derived from [`DependencyName`].
    ///
    /// The [`DependencyConfig`] must be filtered for the current operating
    /// system and package manager beforehad.
    pub fn get_install_package_command(
        &self,
        dep_config: &DependencyConfig,
        interactive: bool,
    ) -> Result<Option<Vec<String>>, Error> {
        let os = dep_config.os.unwrap_or(self.os);
        let pm = dep_config.manager.unwrap_or(self.manager);
        let pm_config = pm.get_config();
        let mut args = vec![];

        let Some(base_args) = pm_config.commands.get(&CommandType::InstallPackage) else {
            return Ok(None);
        };

        for arg in base_args {
            if arg == "$" {
                args.extend(self.extract_package_args(dep_config, &pm_config, &pm, &os)?);
            } else {
                args.push(arg.to_owned());
            }
        }

        self.append_interactive(
            CommandType::InstallPackage,
            &pm_config,
            &mut args,
            interactive,
        );

        Ok(Some(args))
    }

    /// Return the command and arguments to "install many packages" for the
    /// current package manager. Will replace `$` in an argument with the
    /// dependency name, derived from [`DependencyName`].
    ///
    /// The [`DependencyConfig`]s must be filtered for the current operating
    /// system and package manager beforehad.
    pub fn get_install_packages_command(
        &self,
        dep_configs: &[DependencyConfig],
        interactive: bool,
    ) -> Result<Option<Vec<String>>, Error> {
        let pm_config = self.manager.get_config();
        let mut args = vec![];

        let Some(base_args) = pm_config.commands.get(&CommandType::InstallPackage) else {
            return Ok(None);
        };

        for arg in base_args {
            if arg == "$" {
                for dep_config in dep_configs {
                    args.extend(self.extract_package_args(
                        dep_config,
                        &pm_config,
                        &self.manager,
                        &self.os,
                    )?);
                }
            } else {
                args.push(arg.to_owned());
            }
        }

        self.append_interactive(
            CommandType::InstallPackage,
            &pm_config,
            &mut args,
            interactive,
        );

        Ok(Some(args))
    }

    /// Return the command and arguments to "update the registry index"
    /// for the current package manager.
    pub fn get_update_index_command(&self, interactive: bool) -> Option<Vec<String>> {
        let pm_config = self.manager.get_config();

        if let Some(args) = pm_config.commands.get(&CommandType::UpdateIndex) {
            let mut args = args.to_owned();

            self.append_interactive(CommandType::UpdateIndex, &pm_config, &mut args, interactive);

            return Some(args);
        }

        None
    }

    /// Resolve and reduce the dependencies to a list that's applicable
    /// to the current system.
    pub fn resolve_dependencies(&self, deps: &[SystemDependency]) -> Vec<DependencyConfig> {
        let mut configs = vec![];

        for dep in deps {
            let config = dep.to_config();

            if config.os.as_ref().is_some_and(|o| o != &self.os) {
                continue;
            }

            if config.arch.as_ref().is_some_and(|a| a != &self.arch) {
                continue;
            }

            configs.push(config);
        }

        configs
    }

    fn append_interactive(
        &self,
        command: CommandType,
        config: &PackageManagerConfig,
        args: &mut Vec<String>,
        interactive: bool,
    ) {
        if config.prompt_for.contains(&command) {
            match &config.prompt_arg {
                PromptArgument::None => {}
                PromptArgument::Interactive(i) => {
                    if interactive {
                        args.push(i.to_owned());
                    }
                }
                PromptArgument::Skip(y) => {
                    if !interactive {
                        args.push(y.to_owned());
                    }
                }
            };
        }
    }

    fn extract_package_args(
        &self,
        dep_config: &DependencyConfig,
        pm_config: &PackageManagerConfig,
        pm: &SystemPackageManager,
        os: &SystemOS,
    ) -> Result<Vec<String>, Error> {
        let mut args = vec![];

        for dep in dep_config.get_package_names(&os, &pm)? {
            if let Some(ver) = &dep_config.version {
                match &pm_config.version_arg {
                    VersionArgument::None => {
                        args.push(dep);
                    }
                    VersionArgument::Inline(op) => {
                        args.push(format!("{dep}{op}{ver}"));
                    }
                    VersionArgument::Separate(opt) => {
                        args.push(dep);
                        args.push(opt.to_owned());
                        args.push(ver.to_owned());
                    }
                };
            } else {
                args.push(dep);
            }
        }

        Ok(args)
    }
}
