use crate::printer::{format_env_var, Printer};
use crate::session::ProtoSession;
use proto_pdk_api::{HostArch, HostOS};
use starbase::AppResult;
use starbase_styles::color;
use std::env;

#[tracing::instrument(skip_all)]
pub async fn env(session: ProtoSession) -> AppResult {
    let env = &session.env;
    let manager = env.load_config_manager()?;
    let mut printer = Printer::new();

    // STORE

    printer.named_section("Store", |p| {
        p.entry("Root", color::path(&env.store.dir));
        p.entry("Bins", color::path(&env.store.bin_dir));
        p.entry("Shims", color::path(&env.store.shims_dir));
        p.entry("Plugins", color::path(&env.store.plugins_dir));
        p.entry("Tools", color::path(&env.store.inventory_dir));
        p.entry("Temp", color::path(&env.store.temp_dir));

        Ok(())
    })?;

    // ENV

    printer.named_section("Environment", |p| {
        p.entry(
            "Proto version",
            color::muted_light(env!("CARGO_PKG_VERSION")),
        );
        p.entry(
            "Operating system",
            color::muted_light(HostOS::from_env().to_string()),
        );
        p.entry(
            "Architecture",
            color::muted_light(HostArch::from_env().to_string()),
        );
        p.entry_map(
            "Virtual paths",
            env.get_virtual_paths()
                .iter()
                .map(|(h, g)| (color::file(g.to_string_lossy()), color::path(h))),
            None,
        );
        p.entry_list(
            "Configs",
            manager.files.iter().filter_map(|f| {
                if f.exists {
                    Some(color::path(&f.path))
                } else {
                    None
                }
            }),
            None,
        );
        p.entry_map(
            "Variables",
            env::vars().filter_map(|(k, v)| {
                if k.starts_with("PROTO_") {
                    Some((color::property(k), format_env_var(&v)))
                } else {
                    None
                }
            }),
            None,
        );

        Ok(())
    })?;

    printer.flush();

    Ok(())
}
