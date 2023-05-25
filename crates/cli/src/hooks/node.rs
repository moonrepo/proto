use crate::tools::{create_tool, ToolType};
use proto_core::{color, ProtoError};

pub async fn pre_run(tool_type: ToolType, args: &[String]) -> Result<(), ProtoError> {
    let mut is_install_command = false;
    let mut is_global = false;

    match tool_type {
        // npm install -g <dep>
        // pnpm add -g <dep>
        ToolType::Npm | ToolType::Pnpm => {
            is_install_command = args[0] == "install" || args[0] == "i" || args[0] == "add";

            for arg in args {
                if arg == "--global" || arg == "-g" || arg == "--location=global" {
                    is_global = true;
                    break;
                }
            }
        }

        // yarn global add <dep>
        ToolType::Yarn => {
            is_global = args[0] == "global";
            is_install_command = args[1] == "add";
        }
        _ => {}
    };

    if is_install_command && is_global {
        let tool = create_tool(&tool_type).await?;

        return Err(ProtoError::Message(format!(
            "Global binaries must be installed with {} and {} should be added to your {}!\nLearn more: {}",
            color::shell(format!("proto install-global {}", tool.get_id())),
            color::path(tool.get_globals_bin_dir()?),
            color::symbol("PATH"),
						color::url("https://moonrepo.dev/docs/proto/faq#how-can-i-install-a-global-binary-for-a-language")
        )))?;
    }

    Ok(())
}
