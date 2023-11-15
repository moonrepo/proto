use super::list_plugins::print_locator;
use clap::Args;
use proto_core::{detect_version, load_tool, Id, PluginLocator};
use serde::Serialize;
use starbase::system;
use starbase_styles::color::{self, OwoStyle};
use std::io::{BufWriter, Write};

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

    let stdout = std::io::stdout();
    let mut buffer = BufWriter::new(stdout.lock());

    writeln!(
        buffer,
        "{} {} {}",
        OwoStyle::new().bold().style(color::id(&tool.id)),
        color::muted("-"),
        color::muted_light(&tool.metadata.name),
    )
    .unwrap();

    writeln!(
        buffer,
        "\n{}",
        OwoStyle::new()
            .bold()
            .style(color::muted_light("Inventory"))
    )
    .unwrap();

    writeln!(buffer, "  Store: {}", color::path(tool.get_inventory_dir())).unwrap();
    writeln!(
        buffer,
        "  Executable: {}",
        color::path(tool.get_exe_path()?)
    )
    .unwrap();

    if let Some(dir) = tool.get_globals_bin_dir() {
        writeln!(buffer, "  Globals directory: {}", color::path(dir)).unwrap();
    }

    if let Some(prefix) = tool.get_globals_prefix() {
        writeln!(buffer, "  Globals prefix: {}", color::property(prefix)).unwrap();
    }

    let bins = tool.get_bin_locations()?;

    if bins.is_empty() {
        writeln!(buffer, "  Binaries: {}", color::failure("None")).unwrap();
    } else {
        writeln!(buffer, "  Binaries:").unwrap();

        for bin in bins {
            writeln!(
                buffer,
                "    {} {} {}",
                color::muted("-"),
                color::path(bin.path),
                if bin.primary {
                    color::muted_light("(primary)")
                } else {
                    "".into()
                }
            )
            .unwrap();
        }
    }

    let shims = tool.get_shim_locations()?;

    if shims.is_empty() {
        writeln!(buffer, "  Shims: {}", color::failure("None")).unwrap();
    } else {
        writeln!(buffer, "  Shims:").unwrap();

        for shim in shims {
            writeln!(
                buffer,
                "    {} {} {}",
                color::muted("-"),
                color::path(shim.path),
                if shim.primary {
                    color::muted_light("(primary)")
                } else {
                    "".into()
                }
            )
            .unwrap();
        }
    }

    writeln!(
        buffer,
        "\n{}",
        OwoStyle::new().bold().style(color::muted_light("Plugin"))
    )
    .unwrap();

    if let Some(version) = &tool.metadata.plugin_version {
        writeln!(buffer, "  Version: {}", color::hash(version)).unwrap();
    }

    if let Some(locator) = &tool.locator {
        print_locator(&mut buffer, locator);
    }

    writeln!(buffer, "").unwrap();

    buffer.flush().unwrap();
}
