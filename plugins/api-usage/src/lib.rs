use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use std::collections::HashMap;

#[host_fn]
extern "ExtismHost" {
    fn download_file(input: Json<DownloadFileInput>) -> Json<DownloadFileOutput>;
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn get_env_var(name: String) -> String;
    fn host_log(input: Json<HostLogInput>);
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
    fn set_env_var(name: String, value: String);
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct WasmTestConfig {
    number: usize,
    string: String,
    boolean: bool,
    unknown: Option<usize>,
    list: Vec<String>,
    map: HashMap<String, usize>,
    prebuilt_url: String,
    prebuilt_variant: Option<String>,
}

#[plugin_fn]
pub fn testing_macros(_: ()) -> FnResult<()> {
    // Errors
    let _ = plugin_err!(PluginError::Message("Error".into()));
    let _ = plugin_err!(code = 2, "Error");
    let _ = plugin_err!(code = 3, "Error {}", "arg");
    let _ = plugin_err!("Error");
    let _ = plugin_err!("Error {}", "arg");

    // Commands
    let args = ["a", "b", "c"];

    exec_command!("git");
    exec_command!("git", args);
    exec_command!("git", ["a", "b", "c"]);
    exec_command!(input, ExecCommandInput::default());
    exec_command!(pipe, "git");
    exec_command!(pipe, "git", args);
    exec_command!(pipe, "git", ["a", "b", "c"]);
    exec_command!(inherit, "git");
    exec_command!(inherit, "git", args);
    exec_command!(inherit, "git", ["a", "b", "c"]);
    let _ = exec_command!(raw, ExecCommandInput::default());
    let _ = exec_command!(raw, "git");
    let _ = exec_command!(raw, "git", args);
    let _ = exec_command!(raw, "git", ["a", "b", "c"]);

    // Requests
    send_request!("https://some/url");
    send_request!(input, SendRequestInput::new("https://some/url"));
    send_request!(input, SendRequestInput::post("https://some/url"));

    // Downloads
    download_file!("https://some/url", "/proto/temp/file");
    download_file!(
        input,
        DownloadFileInput::new("https://some/url", "/proto/temp/file")
    );

    // Env vars
    let name = "VAR";

    let _ = host_env!("VAR");
    let _ = host_env!(name);
    host_env!("VAR", "value");
    host_env!("VAR", name);
    host_env!(name, name);
    host_env!(name, "value");

    // Logging
    host_log!("Message");
    host_log!("Message {} {} {}", 1, 2, 3);
    host_log!(input, HostLogInput::default());
    host_log!(stdout, "Message");
    host_log!(stdout, "Message {} {} {}", 1, 2, 3);
    host_log!(stderr, "Message");
    host_log!(stderr, "Message {} {} {}", 1, 2, 3);

    Ok(())
}

#[plugin_fn]
pub fn testing_download_file(
    Json(input): Json<DownloadFileInput>,
) -> FnResult<Json<DownloadFileOutput>> {
    Ok(Json(download(input)?))
}

#[plugin_fn]
pub fn testing_get_env_vars(
    Json(names): Json<Vec<String>>,
) -> FnResult<Json<HashMap<String, String>>> {
    Ok(Json(get_host_env_vars(names)?.into_iter().collect()))
}

#[plugin_fn]
pub fn testing_set_env_vars(Json(vars): Json<HashMap<String, String>>) -> FnResult<()> {
    set_host_env_vars(vars)?;

    Ok(())
}

#[plugin_fn]
pub fn testing_send_request(
    Json(input): Json<SendRequestInput>,
) -> FnResult<Json<SendRequestOutput>> {
    Ok(Json(fetch(input)?))
}

#[plugin_fn]
pub fn testing_echo(Json(input): Json<String>) -> FnResult<Json<String>> {
    Ok(Json(input))
}

#[plugin_fn]
pub fn testing_exec_command(
    Json(input): Json<ExecCommandInput>,
) -> FnResult<Json<ExecCommandOutput>> {
    Ok(Json(exec(input)?))
}

#[plugin_fn]
pub fn testing_host_environment(_: ()) -> FnResult<Json<HostEnvironment>> {
    Ok(Json(get_host_environment()?.to_owned()))
}

#[plugin_fn]
pub fn testing_to_real_path(Json(path): Json<String>) -> FnResult<Json<Option<String>>> {
    Ok(Json(
        VirtualPath::new(path)
            .to_real_path()?
            .map(|path| path.to_string()),
    ))
}

#[plugin_fn]
pub fn testing_create_virtual_path(Json(path): Json<String>) -> FnResult<Json<String>> {
    Ok(Json(VirtualPath::create(path)?.to_string()))
}

#[plugin_fn]
pub fn testing_load_git_tags(Json(url): Json<String>) -> FnResult<Json<Vec<String>>> {
    Ok(Json(load_git_tags(url)?))
}

// Doesn't have a `parse_version_file` function on purpose,
// as this tests how proto parses version files by default
#[plugin_fn]
pub fn detect_version_files(
    Json(_): Json<DetectVersionInput>,
) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".api-usage-version".into(), ".api-usage-version.alt".into()],
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn testing_load_tool_versions(
    Json(input): Json<LoadToolVersionsInput>,
) -> FnResult<Json<Vec<String>>> {
    Ok(Json(load_tool_versions(input)?))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    Ok(Json(LoadVersionsOutput::from(vec![
        "1.0.0".into(),
        "1.1.0".into(),
        "2.0.0".into(),
    ])?))
}

// Each platform and version downloads a different file, from the
// URL configured with the `prebuilt-url` setting, and an optional
// build variant configured with the `prebuilt-variant` setting
#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let config = get_tool_config::<WasmTestConfig>()?;

    // Std doesn't create parent directories, so this verifies
    // that the host created the temp directory
    std::fs::write(input.temp_dir.join("download_prebuilt"), "")?;

    let variant_suffix = config
        .prebuilt_variant
        .as_ref()
        .map(|variant| format!("-{variant}"))
        .unwrap_or_default();

    Ok(Json(DownloadPrebuiltOutput {
        download_url: format!(
            "{}/{}/{}-{}-{}{variant_suffix}.txt",
            config.prebuilt_url, input.context.version, env.arch, env.os, env.libc
        ),
        variant: config.prebuilt_variant,
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn register_tool(_: ()) -> FnResult<Json<RegisterToolOutput>> {
    initialize_tracing();

    host_log!(stdout, "Registering tool");
    tracing::error!(num = 123, "Error");
    tracing::warn!(bool = true, "Warning");
    tracing::info!(str = "string", "Info");
    tracing::debug!(vec = ?[1,2,3], "Debug");
    tracing::trace!(tup = ?("a", "b", "c"), "Trace");

    let config = get_tool_config::<WasmTestConfig>()?;

    host_log!("Config = {:?}", config);
    tracing::debug!("Config = {:?}", config);

    Ok(Json(RegisterToolOutput {
        name: "WASM API Usage".into(),
        type_of: PluginType::CommandLine,
        ..RegisterToolOutput::default()
    }))
}
