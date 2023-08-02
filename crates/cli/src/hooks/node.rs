use crate::tools::{create_tool, ToolType};
use proto_core::{color, ProtoError, UserConfig};
use std::env;

pub async fn pre_run(
    tool_type: ToolType,
    args: &[String],
    user_config: &UserConfig,
) -> Result<(), ProtoError> {
    if args.len() < 3
        || env::var("PROTO_INSTALL_GLOBAL").is_ok()
        || !user_config.node_intercept_globals
    {
        return Ok(());
    }

    let mut is_install_command = false;
    let mut is_global = false;

    #[allow(irrefutable_let_patterns)]
    if let ToolType::Plugin(id) = &tool_type {
        // npm install -g <dep>
        // pnpm add -g <dep>
        if id == "npm" || id == "pnpm" {
            is_install_command = args[0] == "install" || args[0] == "i" || args[0] == "add";

            for arg in args {
                if arg == "--global" || arg == "-g" || arg == "--location=global" {
                    is_global = true;
                    break;
                }
            }
        }

        // yarn global add <dep>
        if id == "yarn" {
            is_global = args[0] == "global";
            is_install_command = args[1] == "add";
        }
    };

    if is_install_command && is_global {
        let tool = create_tool(&tool_type).await?;

        return Err(ProtoError::Message(format!(
            "Global binaries must be installed with {} and {} should be added to your {}!\nLearn more: {}\n\nOpt-out of this functionality with {}.",
            color::shell(format!("proto install-global {}", tool.get_id())),
            color::path(tool.get_globals_bin_dir()?.unwrap()),
            color::shell("PATH"),
            color::url("https://moonrepo.dev/docs/proto/faq#how-can-i-install-a-global-binary-for-a-language"),
            color::symbol("node-intercept-globals = false")
        )))?;
    }

    Ok(())
}
