use crate::{ProtoConfig, ProtoFileManager};
use proto_shim::get_exe_file_name;
use starbase_sandbox::{Sandbox, assert_cmd};
use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};

pub struct ProtoSandbox {
    pub sandbox: Sandbox,
}

impl ProtoSandbox {
    pub fn new(mut sandbox: Sandbox) -> Self {
        apply_settings(&mut sandbox);

        Self { sandbox }
    }
}

impl Deref for ProtoSandbox {
    type Target = Sandbox;

    fn deref(&self) -> &Self::Target {
        &self.sandbox
    }
}

// Env vars the `ai_env` crate checks when detecting an AI agent. Neutralized
// in test commands (empty values count as not set), otherwise output changes
// when the tests themselves run under an agent. Tests can still opt in
// explicitly, e.g. `.env("CODEX_CI", "1")`.
const AI_AGENT_ENV_VARS: &[&str] = &[
    "AI_AGENT",
    "ANTIGRAVITY_AGENT",
    "AUGMENT_AGENT",
    "CLAUDECODE",
    "CLAUDE_CODE",
    "CLAUDE_CODE_IS_COWORK",
    "CODEX_CI",
    "CODEX_SANDBOX",
    "CODEX_THREAD_ID",
    "COPILOT_ALLOW_ALL",
    "COPILOT_CLI",
    "COPILOT_GITHUB_TOKEN",
    "COPILOT_MODEL",
    "CURSOR_AGENT",
    "CURSOR_EXTENSION_HOST_ROLE",
    "CURSOR_TRACE_ID",
    "GEMINI_CLI",
    "OPENCODE",
    "OPENCODE_CLIENT",
    "REPL_ID",
];

fn apply_settings(sandbox: &mut Sandbox) {
    let root = sandbox.path().to_path_buf();
    let home_dir = root.join(".home");
    let proto_dir = root.join(".proto");

    // Folders must exist or tests fail!
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&proto_dir).unwrap();

    // Tool plugins read these through `get_env_var` and fail when the
    // paths cannot be mapped into the sandbox's virtual paths, so keep
    // them within the sandboxed home directory.
    let rustup_dir = home_dir.join(".rustup");
    let cargo_dir = home_dir.join(".cargo");

    let mut env = HashMap::new();
    env.insert("RUSTUP_HOME", rustup_dir.to_str().unwrap());
    env.insert("CARGO_HOME", cargo_dir.to_str().unwrap());
    env.insert("RUST_BACKTRACE", "1");
    env.insert("WASMTIME_BACKTRACE_DETAILS", "1");
    env.insert("NO_COLOR", "1");
    env.insert("PROTO_SANDBOX", root.to_str().unwrap());
    env.insert("PROTO_HOME", proto_dir.to_str().unwrap());
    env.insert("PROTO_LOG", "trace");
    // Keep CLI output deterministic across developer shell environments.
    env.insert("PROTO_JSON", "false");
    env.insert("PROTO_REPORTER", "text");
    env.insert("PROTO_TEST", "true");

    for key in AI_AGENT_ENV_VARS {
        env.insert(*key, "");
    }

    sandbox.settings.bin = "proto".into();
    sandbox.settings.timeout = 300;

    sandbox
        .settings
        .env
        .extend(env.into_iter().map(|(k, v)| (k.to_owned(), v.to_owned())));
}

pub fn create_empty_proto_sandbox() -> ProtoSandbox {
    ProtoSandbox::new(starbase_sandbox::create_empty_sandbox())
}

pub fn create_proto_sandbox<N: AsRef<str>>(fixture: N) -> ProtoSandbox {
    ProtoSandbox::new(starbase_sandbox::create_sandbox(fixture))
}

/// A plugins directory shared by every sandbox that opts into it, so that
/// commands loading *all* tools don't each re-download the same builtin
/// plugins. Machine local and reused across runs; the loader coordinates
/// concurrent access through lock files kept alongside it.
pub fn shared_plugins_dir() -> PathBuf {
    std::env::temp_dir().join("proto-test-plugins")
}

fn apply_shared_plugins(sandbox: &mut Sandbox) {
    let dir = shared_plugins_dir();

    fs::create_dir_all(&dir).unwrap();

    sandbox.settings.env.insert(
        "PROTO_PLUGINS_DIR".into(),
        dir.to_str().unwrap().to_owned(),
    );
}

/// Like [`create_empty_proto_sandbox`], but points the sandbox at a plugins
/// directory shared with other tests. Only for tests that run commands which
/// load every configured tool (`clean`, `exec`, `outdated`, `status`,
/// `install` with no argument, ...), where the builtin plugin downloads
/// otherwise dominate the runtime. Do NOT use it for tests that assert on or
/// mutate the contents of the plugins directory.
pub fn create_empty_proto_sandbox_with_shared_plugins() -> ProtoSandbox {
    let mut sandbox = starbase_sandbox::create_empty_sandbox();

    apply_shared_plugins(&mut sandbox);

    ProtoSandbox::new(sandbox)
}

/// See [`create_empty_proto_sandbox_with_shared_plugins`].
pub fn create_proto_sandbox_with_shared_plugins<N: AsRef<str>>(fixture: N) -> ProtoSandbox {
    let mut sandbox = starbase_sandbox::create_sandbox(fixture);

    apply_shared_plugins(&mut sandbox);

    ProtoSandbox::new(sandbox)
}

pub fn load_config<T: AsRef<Path>>(dir: T) -> ProtoConfig {
    let manager = ProtoFileManager::load(dir, None, None).unwrap();
    let config = manager.get_merged_config().unwrap();
    config.to_owned()
}

pub fn create_shim_command<T: AsRef<Path>>(path: T, name: &str) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::from_std(create_shim_command_std(path, name));
    cmd.timeout(std::time::Duration::from_secs(240));
    cmd
}

pub fn create_shim_command_std<T: AsRef<Path>>(path: T, name: &str) -> std::process::Command {
    let path = path.as_ref();

    let mut cmd = std::process::Command::new(get_shim_path(path, name));
    cmd.env("PROTO_LOG", "trace");
    cmd.env("PROTO_HOME", path.join(".proto"));
    cmd.env("PROTO_NODE_VERSION", "*"); // For package managers
    cmd.env(format!("PROTO_{}_VERSION", name.to_uppercase()), "*");

    for key in AI_AGENT_ENV_VARS {
        cmd.env(key, "");
    }

    cmd
}

pub fn get_bin_path<T: AsRef<Path>>(path: T, name: &str) -> PathBuf {
    path.as_ref()
        .join(".proto/bin")
        .join(get_exe_file_name(name))
}

pub fn get_shim_path<T: AsRef<Path>>(path: T, name: &str) -> PathBuf {
    path.as_ref()
        .join(".proto/shims")
        .join(get_exe_file_name(name))
}

pub fn link_bin(input_path: &Path, output_path: &Path) {
    fs::create_dir_all(output_path.parent().unwrap()).unwrap();

    #[cfg(windows)]
    {
        fs::copy(input_path, output_path).unwrap();
    }

    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(input_path, output_path).unwrap();
    }
}
