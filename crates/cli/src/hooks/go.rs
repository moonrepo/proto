use crate::shell;
use starbase_styles::color;
use tracing::info;

pub fn post_install(passthrough: &[String]) -> miette::Result<()> {
    if passthrough.contains(&"--no-gobin".to_string()) {
        return Ok(());
    }

    let shell = shell::detect_shell(None);
    let env_vars = vec![
        ("GOBIN".to_string(), "$HOME/go/bin".to_string()),
        ("PATH".to_string(), "$GOBIN".to_string()),
    ];

    if let Some(content) = shell::format_env_vars(&shell, "go", env_vars) {
        if let Some(updated_profile) = shell::write_profile_if_not_setup(&shell, content, "GOBIN")?
        {
            info!(
                "Added GOBIN to your shell profile {}",
                color::path(updated_profile)
            );
        }
    }

    Ok(())
}
