use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn from_virtual_path(path: String) -> String;
    fn get_env_var(name: String) -> String;
    fn host_log(input: Json<HostLogInput>);
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
    fn set_env_var(name: String, value: String);
    fn to_virtual_path(path: String) -> String;
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

    // Paths
    let path = "/proto/path";
    let pathbuf = PathBuf::from("/proto/buf");

    let _ = real_path!("/proto/dir");
    let _ = real_path!(path);
    let _ = real_path!(buf, pathbuf);
    let _ = virtual_path!("/proto/dir");
    let _ = virtual_path!(path);
    let _ = virtual_path!(buf, pathbuf);

    Ok(())
}

#[plugin_fn]
pub fn register_tool(_: ()) -> FnResult<Json<RegisterToolOutput>> {
    host_log!(stdout, "Registering tool");
    tracing::error!("Error");
    tracing::warn!("Warning");
    tracing::info!("Info");
    tracing::debug!("Debug");
    tracing::trace!("Trace");

    let config = get_tool_config::<WasmTestConfig>()?;

    host_log!("Config = {:?}", config);
    tracing::debug!("Config = {:?}", config);

    Ok(Json(RegisterToolOutput {
        name: "WASM API Usage".into(),
        type_of: PluginType::CommandLine,
        ..RegisterToolOutput::default()
    }))
}
