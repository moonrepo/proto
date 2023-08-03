use crate::tools::create_tool;
use proto_core::{AliasOrVersion, ProtoError};
use starbase::SystemResult;
use starbase_styles::color;
use tracing::info;

pub async fn alias(
    tool_id: String,
    alias: AliasOrVersion,
    version: AliasOrVersion,
) -> SystemResult {
    let mut tool = create_tool(&tool_id).await?;

    match &alias {
        AliasOrVersion::Alias(alias) => {
            if let AliasOrVersion::Alias(to_alias) = &version {
                if alias == to_alias {
                    return Err(ProtoError::Message("Cannot map an alias to itself.".into()))?;
                }
            }

            tool.manifest
                .aliases
                .insert(alias.to_owned(), version.clone());
            tool.manifest.save()?;
        }
        AliasOrVersion::Version(_) => {
            return Err(ProtoError::Message(
                "Versions cannot be aliases. Use alphanumeric words instead.".into(),
            ))?;
        }
    };

    info!(
        "Added alias {} ({}) for {}",
        color::id(alias.to_string()),
        color::muted_light(version.to_string()),
        tool.get_name(),
    );

    Ok(())
}
