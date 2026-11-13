use extism_pdk::Json;
use proto_pdk_api::{AnyResult, LoadToolVersionsInput, LoadToolVersionsOutput};

mod host_functions {
    use extism_pdk::{Json, host_fn};
    use proto_pdk_api::{LoadToolVersionsInput, LoadToolVersionsOutput};

    #[host_fn]
    extern "ExtismHost" {
        pub(super) fn load_tool_versions(
            input: Json<LoadToolVersionsInput>,
        ) -> Json<LoadToolVersionsOutput>;
    }
}

/// Load available versions from the provided Git or JSON index source.
pub fn load_tool_versions(input: LoadToolVersionsInput) -> AnyResult<LoadToolVersionsOutput> {
    Ok(unsafe { host_functions::load_tool_versions(Json(input))?.0 })
}

/// Load tool versions from a Git remote.
pub fn load_git_tags<U>(url: U) -> AnyResult<Vec<String>>
where
    U: AsRef<str>,
{
    Ok(load_tool_versions(LoadToolVersionsInput::Git {
        url: url.as_ref().to_owned(),
        tag_pattern: None,
    })?
    .versions)
}
