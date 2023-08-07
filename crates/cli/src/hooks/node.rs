use crate::tools::create_tool;
use proto_core::{ProtoError, UserConfig};
use starbase_styles::color;
use std::env;

pub async fn pre_run(
    tool_id: &str,
    args: &[String],
    user_config: &UserConfig,
) -> miette::Result<()> {
    if args.len() < 3
        || env::var("PROTO_INSTALL_GLOBAL").is_ok()
        || !user_config.node_intercept_globals
    {
        return Ok(());
    }

    let mut is_install_command = false;
    let mut is_global = false;

    // npm install -g <dep>
    // pnpm add -g <dep>
    if tool_id == "npm" || tool_id == "pnpm" {
        is_install_command = args[0] == "install" || args[0] == "i" || args[0] == "add";

        for arg in args {
            if arg == "--global" || arg == "-g" || arg == "--location=global" {
                is_global = true;
                break;
            }
        }
    }

    // yarn global add <dep>
    if tool_id == "yarn" {
        is_global = args[0] == "global";
        is_install_command = args[1] == "add";
    }

    if is_install_command && is_global {
        let mut tool = create_tool(tool_id).await?;
        tool.locate_globals_dir().await?;

        return Err(ProtoError::Message(format!(
            "Global binaries must be installed with {} and {} should be added to your {}!\nLearn more: {}\n\nOpt-out of this functionality with {}.",
            color::shell(format!("proto install-global {}", tool.id)),
            color::path(tool.get_globals_bin_dir().unwrap()),
            color::shell("PATH"),
            color::url("https://moonrepo.dev/docs/proto/faq#how-can-i-install-a-global-binary-for-a-language"),
            color::symbol("node-intercept-globals = false")
        )))?;
    }

    Ok(())
}
