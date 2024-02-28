use crate::helpers::ProtoResource;
use crate::printer::{format_env_var, format_value, Printer};
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{
    detect_version, EnvVar, ExecutableLocation, Id, PluginLocator, ProtoToolConfig, ToolManifest,
    UnresolvedVersionSpec,
};
use proto_pdk_api::ToolMetadataOutput;
use serde::Serialize;
use starbase::system;
use starbase_styles::color;
use starbase_utils::json;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct PluginInfo {
    bins: Vec<ExecutableLocation>,
    config: ProtoToolConfig,
    exe_path: PathBuf,
    globals_dir: Option<PathBuf>,
    globals_prefix: Option<String>,
    id: Id,
    inventory_dir: PathBuf,
    manifest: ToolManifest,
    metadata: ToolMetadataOutput,
    name: String,
    plugin: PluginLocator,
    shims: Vec<ExecutableLocation>,
}

#[derive(Args, Clone, Debug)]
pub struct PluginInfoArgs {
    #[arg(required = true, help = "ID of plugin")]
    id: Id,

    #[arg(long, help = "Print the info in JSON format")]
    json: bool,
}

#[system]
pub async fn info(args: ArgsRef<PluginInfoArgs>, proto: ResourceRef<ProtoResource>) {
    let mut tool = proto.load_tool(&args.id).await?;
    let version = detect_version(&tool, None).await?;

    tool.resolve_version(&version, false).await?;
    tool.create_executables(false, false).await?;
    tool.locate_globals_dir().await?;

    let mut config = proto.env.load_config()?.to_owned();
    let tool_config = config.tools.remove(&tool.id).unwrap_or_default();

    if args.json {
        let info = PluginInfo {
            bins: tool.get_bin_locations()?,
            config: tool_config,
            exe_path: tool.get_exe_path()?.to_path_buf(),
            globals_dir: tool.get_globals_bin_dir().map(|p| p.to_path_buf()),
            globals_prefix: tool.get_globals_prefix().map(|p| p.to_owned()),
            inventory_dir: tool.get_inventory_dir(),
            shims: tool.get_shim_locations()?,
            id: tool.id,
            name: tool.metadata.name.clone(),
            manifest: tool.manifest,
            metadata: tool.metadata,
            plugin: tool.locator.unwrap(),
        };

        println!("{}", json::to_string_pretty(&info).into_diagnostic()?);

        return Ok(());
    }

    let latest_version = UnresolvedVersionSpec::default();
    let mut version_resolver = tool.load_version_resolver(&latest_version).await?;
    version_resolver.aliases.extend(tool_config.aliases.clone());

    let mut printer = Printer::new();
    printer.header(&tool.id, &tool.metadata.name);

    // PLUGIN

    printer.named_section("Plugin", |p| {
        if let Some(version) = &tool.metadata.plugin_version {
            p.entry("Version", color::hash(version));
        }

        p.locator(tool.locator.as_ref().unwrap());

        Ok(())
    })?;

    // INVENTORY

    printer.named_section("Inventory", |p| {
        p.entry("Store", color::path(tool.get_inventory_dir()));

        p.entry("Executable", color::path(tool.get_exe_path()?));

        if let Some(dir) = tool.get_globals_bin_dir() {
            p.entry("Globals directory", color::path(dir));
        }

        if let Some(prefix) = tool.get_globals_prefix() {
            p.entry("Globals prefix", color::property(prefix));
        }

        p.entry_list(
            "Binaries",
            tool.get_bin_locations()?.into_iter().map(|bin| {
                format!(
                    "{} {}",
                    color::path(bin.path),
                    if bin.primary {
                        color::muted_light("(primary)")
                    } else {
                        "".into()
                    }
                )
            }),
            Some(color::failure("None")),
        );

        p.entry_list(
            "Shims",
            tool.get_shim_locations()?.into_iter().map(|shim| {
                format!(
                    "{} {}",
                    color::path(shim.path),
                    if shim.primary {
                        format_value("(primary)")
                    } else {
                        "".into()
                    }
                )
            }),
            Some(color::failure("None")),
        );

        let mut versions = tool.manifest.installed_versions.iter().collect::<Vec<_>>();
        versions.sort();

        p.entry_list(
            "Installed versions",
            versions
                .iter()
                .map(|version| color::hash(version.to_string())),
            Some(color::failure("None")),
        );

        if !version_resolver.aliases.is_empty() {
            p.entry_map(
                "Aliases",
                version_resolver
                    .aliases
                    .iter()
                    .map(|(k, v)| (color::hash(k), format_value(v.to_string())))
                    .collect::<Vec<_>>(),
                None,
            );
        }

        Ok(())
    })?;

    // CONFIG

    if !tool_config.aliases.is_empty()
        || !tool_config.env.is_empty()
        || !tool_config.config.is_empty()
    {
        printer.named_section("Configuration", |p| {
            p.entry_map(
                "Aliases",
                tool_config
                    .aliases
                    .iter()
                    .map(|(k, v)| (color::hash(k), format_value(v.to_string()))),
                None,
            );

            p.entry_map(
                "Environment variables",
                tool_config.env.iter().map(|(k, v)| {
                    (
                        color::property(k),
                        match v {
                            EnvVar::State(state) => {
                                if *state {
                                    format_value("true")
                                } else {
                                    color::muted("(removed)")
                                }
                            }
                            EnvVar::Value(value) => format_env_var(value),
                        },
                    )
                }),
                None,
            );

            p.entry_map(
                "Settings",
                tool_config
                    .config
                    .iter()
                    .map(|(k, v)| (k, format_value(v.to_string()))),
                None,
            );

            Ok(())
        })?;
    }

    printer.flush();
}
