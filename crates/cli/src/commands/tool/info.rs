use crate::printer::Printer;
use clap::Args;
use proto_core::{detect_version, load_tool, Id, PluginLocator};
use serde::Serialize;
use starbase::system;
use starbase_styles::color;

#[derive(Serialize)]
pub struct PluginItem {
    id: Id,
    locator: PluginLocator,
    name: String,
    version: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ToolInfoArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,
}

#[system]
pub async fn tool_info(args: ArgsRef<ToolInfoArgs>) {
    let mut tool = load_tool(&args.id).await?;
    let version = detect_version(&tool, None).await?;

    tool.resolve_version(&version, false).await?;
    tool.create_executables(false, false).await?;
    tool.locate_globals_dir().await?;

    let mut printer = Printer::new();

    printer.header(&tool.id, &tool.metadata.name);

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
                        color::muted_light("(primary)")
                    } else {
                        "".into()
                    }
                )
            }),
            Some(color::failure("None")),
        );

        Ok(())
    })?;

    // PLUGIN

    printer.named_section("Plugin", |p| {
        if let Some(version) = &tool.metadata.plugin_version {
            p.entry("Version", color::hash(version));
        }

        p.locator(tool.locator.as_ref().unwrap());

        Ok(())
    })?;

    printer.flush();
}
