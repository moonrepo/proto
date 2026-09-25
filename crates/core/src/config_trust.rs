use crate::config::{
    PROTO_PLUGIN_KEY, PartialProtoConfig, PartialProtoSettingsConfig, PartialProtoToolConfig,
};
use crate::config_error::ProtoConfigError;
use crate::id::Id;
use crate::tool_context::ToolContext;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use starbase_utils::json::{self, JsonError, JsonValue};
use starbase_utils::{fs, hash};
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};

/// The trust state of a config file.
///
/// A config that is not owned by the user, like one in a cloned repository,
/// can execute code on the host through its security-sensitive settings:
/// environment variables and shell aliases that are applied to the shell,
/// plugins that are loaded and executed, and settings that change where
/// plugins and tools are downloaded from. These settings are only applied
/// once the directory of the config has been trusted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProtoConfigTrust {
    /// The config is owned by the user (user or global config),
    /// or does not contain security-sensitive settings.
    #[default]
    NotRequired,

    /// The security-sensitive settings are applied.
    Trusted,

    /// The security-sensitive settings have been ignored.
    Untrusted,
}

/// Return the paths of the security-sensitive settings configured in the
/// provided config, for display purposes. Returns an empty list if the
/// config has none, in which case it does not need to be trusted.
pub fn get_sensitive_fields(config: &PartialProtoConfig) -> Result<Vec<String>, ProtoConfigError> {
    let (_, sensitive) = split_config(config.to_owned());

    let value = json::serde_json::to_value(&sensitive).map_err(|error| {
        Box::new(JsonError::Format {
            error: Box::new(error),
        })
    })?;

    Ok(prune_json(value)
        .map(|value| collect_fields(&value))
        .unwrap_or_default())
}

/// Split a config into the settings that are safe to apply from any config,
/// and the settings that are security-sensitive and require trust.
///
/// This is an allowlist, so that settings added in the future are
/// treated as sensitive until deemed otherwise.
pub fn split_config(mut config: PartialProtoConfig) -> (PartialProtoConfig, PartialProtoConfig) {
    let mut safe = PartialProtoConfig::default();

    // Version pins are safe, except for proto itself, as it determines which
    // proto binary is executed, and a version released before trust existed
    // would not enforce it
    if let Some(mut versions) = config.versions.take() {
        let proto_context = ToolContext::new(Id::raw(PROTO_PLUGIN_KEY));

        if let Some(spec) = versions.remove(&proto_context) {
            config.versions = Some(BTreeMap::from_iter([(proto_context, spec)]));
        }

        safe.versions = Some(versions);
    }

    // Versions also show up in the unknown fields, because of flattening
    if let Some(mut unknown) = config.unknown.take() {
        if let Some(value) = unknown.remove(PROTO_PLUGIN_KEY) {
            config.unknown = Some(FxHashMap::from_iter([(PROTO_PLUGIN_KEY.to_owned(), value)]));
        }

        safe.unknown = Some(unknown);
    }

    // Version aliases are safe, while the tool's environment
    // variables, plugin, and plugin configuration are not
    if let Some(tools) = &mut config.tools {
        for (context, tool) in tools.iter_mut() {
            if let Some(aliases) = tool.aliases.take() {
                safe.tools.get_or_insert_default().insert(
                    context.to_owned(),
                    PartialProtoToolConfig {
                        aliases: Some(aliases),
                        ..Default::default()
                    },
                );
            }
        }
    }

    if let Some(settings) = &mut config.settings {
        safe.settings = Some(PartialProtoSettingsConfig {
            detect_strategy: settings.detect_strategy.take(),
            lockfile: settings.lockfile.take(),
            pin_latest: settings.pin_latest.take(),
            telemetry: settings.telemetry.take(),
            ..Default::default()
        });
    }

    (safe, config)
}

/// Remove nulls and empty objects (settings that were not configured)
/// from a JSON value. Returns `None` if nothing remains.
fn prune_json(value: JsonValue) -> Option<JsonValue> {
    match value {
        JsonValue::Null => None,
        JsonValue::Object(map) => {
            let map = map
                .into_iter()
                .filter_map(|(key, value)| prune_json(value).map(|value| (key, value)))
                .collect::<json::JsonMap<_, _>>();

            (!map.is_empty()).then_some(JsonValue::Object(map))
        }
        other => Some(other),
    }
}

/// Collect the paths of the configured settings, 2 levels deep for tables.
fn collect_fields(value: &JsonValue) -> Vec<String> {
    let mut fields = vec![];

    if let JsonValue::Object(map) = value {
        for (key, value) in map {
            match value {
                JsonValue::Object(inner) if key != "env" => {
                    for inner_key in inner.keys() {
                        fields.push(format!("{key}.{inner_key}"));
                    }
                }
                _ => {
                    fields.push(key.to_owned());
                }
            }
        }
    }

    fields.sort();
    fields
}

/// Why a config file is trusted.
#[derive(Clone, Debug, PartialEq)]
pub enum ProtoTrustSource {
    /// All configs are trusted, as we're running in CI.
    Ci,

    /// Within a directory listed in `PROTO_TRUSTED_PATHS`.
    TrustedPath(PathBuf),

    /// The config file itself, or a directory it's within,
    /// was trusted with `proto trust`.
    Record(PathBuf),
}

/// A record that a config file, or the configs within a directory,
/// have been trusted by the user.
#[derive(Deserialize, Serialize)]
struct ProtoTrustRecord {
    path: PathBuf,
}

/// Determines which config files are trusted. A config is trusted when the
/// user has trusted the file itself or a directory it's within (with
/// `proto trust`), when it's within a directory listed in
/// `PROTO_TRUSTED_PATHS`, or when running in CI.
#[derive(Clone, Debug, Default)]
pub struct ProtoTrustStore {
    /// Directory of trust records: `~/.proto/trust`.
    pub dir: PathBuf,

    /// Trust all config files, regardless of their settings.
    pub trust_all: bool,

    /// Trust all config files within these directories.
    pub trusted_paths: Vec<PathBuf>,
}

impl ProtoTrustStore {
    pub fn new(dir: PathBuf) -> Self {
        // Variables applied to the shell by a previous activation must not
        // influence trust, otherwise a trusted config could trust the next
        // directory that's entered. See `ACTIVATED_ENV_KEY` in the CLI.
        let activated = env::var("_PROTO_ACTIVATED_ENV").unwrap_or_default();
        let from_shell = |key: &str| !activated.split(',').any(|activated| activated == key);

        let mut trusted_paths = vec![];

        if from_shell("PROTO_TRUSTED_PATHS")
            && let Some(value) = env::var_os("PROTO_TRUSTED_PATHS")
        {
            for path in env::split_paths(&value) {
                if !path.as_os_str().is_empty() {
                    trusted_paths.push(normalize_path(&path));
                }
            }
        }

        Self {
            dir,
            // In CI, the checked out repository is the code being
            // built, and there's no user around to trust it
            trust_all: from_shell("CI") && ci_env::is_ci(),
            trusted_paths,
        }
    }

    /// Trust all config files within the provided directory.
    pub fn add_trusted_path(&mut self, dir: &Path) {
        self.trusted_paths.push(normalize_path(dir));
    }

    /// Return why the provided config file is trusted, or `None` if it's not.
    pub fn get_trust_source(&self, config_path: &Path) -> Option<ProtoTrustSource> {
        if self.trust_all {
            trace!(config = ?config_path, "Trusting config as all configs are trusted (CI)");

            return Some(ProtoTrustSource::Ci);
        }

        let config_path = normalize_path(config_path);

        if let Some(dir) = self
            .trusted_paths
            .iter()
            .find(|dir| config_path.starts_with(dir))
        {
            trace!(config = ?config_path, "Trusting config as it's within a trusted path");

            return Some(ProtoTrustSource::TrustedPath(dir.to_owned()));
        }

        // The file itself may be trusted
        if self.has_record(&config_path) {
            trace!(config = ?config_path, "Trusting config as the file is trusted");

            return Some(ProtoTrustSource::Record(config_path));
        }

        // Otherwise trust applies to a directory and everything within it
        let mut current = config_path.parent();

        while let Some(dir) = current {
            if self.has_record(dir) {
                trace!(config = ?config_path, dir = ?dir, "Trusting config as its directory is trusted");

                return Some(ProtoTrustSource::Record(dir.to_owned()));
            }

            current = dir.parent();
        }

        None
    }

    /// Return true if the provided config file is trusted.
    pub fn is_trusted(&self, config_path: &Path) -> bool {
        self.get_trust_source(config_path).is_some()
    }

    /// Trust the provided config file, or the config files within the provided
    /// directory and its sub-directories. Returns the normalized path.
    pub fn trust(&self, path: &Path) -> Result<PathBuf, ProtoConfigError> {
        let path = normalize_path(path);

        debug!(path = ?path, "Trusting path");

        json::write_file(
            self.get_record_path(&path),
            &ProtoTrustRecord { path: path.clone() },
            true,
        )
        .map_err(Box::new)?;

        Ok(path)
    }

    /// Remove trust for the provided config file or directory. Returns true
    /// if it was previously trusted.
    pub fn untrust(&self, path: &Path) -> Result<bool, ProtoConfigError> {
        let path = normalize_path(path);
        let record_path = self.get_record_path(&path);

        if !record_path.exists() {
            return Ok(false);
        }

        debug!(path = ?path, "Untrusting path");

        fs::remove_file(record_path)?;

        Ok(true)
    }

    fn has_record(&self, path: &Path) -> bool {
        let record_path = self.get_record_path(path);

        if !record_path.exists() {
            return false;
        }

        // Treat unreadable records as untrusted
        match json::read_file::<ProtoTrustRecord>(&record_path) {
            Ok(record) => record.path == path,
            Err(error) => {
                debug!(
                    record = ?record_path,
                    error = ?error,
                    "Failed to read trust record, treating path as untrusted"
                );

                false
            }
        }
    }

    // One file per trusted path, so that trusting multiple paths concurrently
    // never has to read, modify, and write a shared file
    fn get_record_path(&self, path: &Path) -> PathBuf {
        self.dir.join(format!("{}.json", hash_path(path)))
    }
}

/// Hash a path, for use as an identifier.
pub fn hash_path(path: &Path) -> String {
    hash::sha256::from_bytes(path.as_os_str().as_encoded_bytes())
}

/// Resolve symlinks and relative components so that a directory is trusted
/// regardless of the path used to reach it. A config file may not exist yet,
/// so fall back to resolving its directory.
pub fn normalize_path(path: &Path) -> PathBuf {
    if let Ok(path) = std::fs::canonicalize(path) {
        return path;
    }

    if let (Some(parent), Some(name)) = (path.parent(), path.file_name())
        && let Ok(parent) = std::fs::canonicalize(parent)
    {
        return parent.join(name);
    }

    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProtoConfig;
    use starbase_sandbox::create_empty_sandbox;

    fn parse(content: &str) -> PartialProtoConfig {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", content);

        ProtoConfig::parse(&sandbox.path().join(".prototools"), false).unwrap()
    }

    fn sensitive(content: &str) -> Vec<String> {
        get_sensitive_fields(&parse(content)).unwrap()
    }

    mod sensitive {
        use super::*;

        #[test]
        fn none_for_empty_config() {
            assert!(sensitive("").is_empty());
        }

        #[test]
        fn none_for_safe_settings() {
            assert!(
                sensitive(
                    r#"
node = "20"
npm = "10.0.0"
"npm:typescript" = "5"

[tools.node.aliases]
work = "18"

[settings]
detect-strategy = "prefer-prototools"
lockfile = true
pin-latest = "local"
telemetry = false
"#
                )
                .is_empty()
            );
        }

        #[test]
        fn none_for_empty_tables() {
            assert!(sensitive("[env]\n[settings]\n[shell]\n[tools.node]\n").is_empty());
        }

        #[test]
        fn includes_env_vars() {
            assert_eq!(sensitive("[env]\nBASH_ENV = \"script.sh\"\n"), ["env"]);
        }

        #[test]
        fn includes_env_files() {
            assert_eq!(sensitive("[env]\nfile = \".env\"\n"), ["env"]);
        }

        #[test]
        fn includes_shell_aliases() {
            assert_eq!(
                sensitive("[shell.aliases]\nls = \"echo\"\n"),
                ["shell.aliases"]
            );
        }

        #[test]
        fn includes_plugins() {
            assert_eq!(
                sensitive(
                    r#"
[plugins.tools]
foo = "file://./foo.wasm"

[tools.bar]
plugin = "https://example.com/bar.wasm"
"#,
                ),
                ["plugins.tools", "tools.bar"]
            );
        }

        #[test]
        fn includes_tool_and_backend_config() {
            assert_eq!(
                sensitive(
                    r#"
[tools.node]
dist-url = "https://example.com"

[tools.node.env]
KEY = "value"

[backends.asdf]
repository = "https://example.com"
"#,
                ),
                ["backends.asdf", "tools.node"]
            );
        }

        #[test]
        fn includes_unsafe_settings() {
            assert_eq!(
                sensitive(
                    r#"
[settings]
auto-install = true
lockfile = true

[settings.url-rewrites]
"github.com" = "example.com"
"#,
                ),
                ["settings.auto-install", "settings.url-rewrites"]
            );
        }

        #[test]
        fn includes_proto_pin() {
            assert_eq!(sensitive("node = \"20\"\nproto = \"0.40.0\"\n"), ["proto"]);
        }
    }

    mod split {
        use super::*;

        #[test]
        fn keeps_safe_settings() {
            let (safe, _) = split_config(parse(
                r#"
node = "20"
proto = "0.40.0"

[env]
KEY = "value"

[tools.node]
dist-url = "https://example.com"

[tools.node.aliases]
work = "18"

[settings]
auto-install = true
lockfile = true
"#,
            ));

            let versions = safe.versions.unwrap();

            assert!(versions.contains_key(&ToolContext::new(Id::raw("node"))));
            assert!(!versions.contains_key(&ToolContext::new(Id::raw("proto"))));
            assert!(safe.env.is_none());
            assert!(safe.plugins.is_none());
            assert!(safe.shell.is_none());
            assert!(safe.backends.is_none());

            let tool = safe
                .tools
                .unwrap()
                .remove(&ToolContext::new(Id::raw("node")))
                .unwrap();

            assert!(tool.aliases.is_some());
            assert!(tool.config.is_none());

            let settings = safe.settings.unwrap();

            assert_eq!(settings.lockfile, Some(true));
            assert_eq!(settings.auto_install, None);
        }
    }

    mod store {
        use super::*;

        fn create_store(sandbox: &Path) -> ProtoTrustStore {
            ProtoTrustStore {
                dir: sandbox.join("trust"),
                trust_all: false,
                trusted_paths: vec![],
            }
        }

        #[test]
        fn not_trusted_by_default() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let store = create_store(sandbox.path());

            assert_eq!(
                store.get_trust_source(&sandbox.path().join(".prototools")),
                None
            );
        }

        #[test]
        fn trusts_directory() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/.prototools", "");
            sandbox.create_file("b/.prototools", "");

            let store = create_store(sandbox.path());
            let dir = store.trust(&sandbox.path().join("a")).unwrap();

            assert_eq!(
                store.get_trust_source(&sandbox.path().join("a/.prototools")),
                Some(ProtoTrustSource::Record(dir))
            );
            assert!(!store.is_trusted(&sandbox.path().join("b/.prototools")));
        }

        #[test]
        fn trusts_nested_directories() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/b/c/.prototools", "");

            let store = create_store(sandbox.path());
            store.trust(&sandbox.path().join("a")).unwrap();

            assert!(store.is_trusted(&sandbox.path().join("a/b/c/.prototools")));
            assert!(store.is_trusted(&sandbox.path().join("a/b/.prototools.prod")));
            assert!(!store.is_trusted(&sandbox.path().join(".prototools")));
        }

        #[test]
        fn trusts_file() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");
            sandbox.create_file(".prototools.prod", "");
            sandbox.create_file("child/.prototools", "");

            let store = create_store(sandbox.path());
            let file = store.trust(&sandbox.path().join(".prototools")).unwrap();

            assert_eq!(
                store.get_trust_source(&sandbox.path().join(".prototools")),
                Some(ProtoTrustSource::Record(file))
            );

            // Only that file
            assert!(!store.is_trusted(&sandbox.path().join(".prototools.prod")));
            assert!(!store.is_trusted(&sandbox.path().join("child/.prototools")));
        }

        #[test]
        fn file_and_directory_trust_are_independent() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let store = create_store(sandbox.path());
            let file = sandbox.path().join(".prototools");

            store.trust(sandbox.path()).unwrap();
            store.trust(&file).unwrap();

            assert!(store.untrust(&file).unwrap());
            assert!(matches!(
                store.get_trust_source(&file),
                Some(ProtoTrustSource::Record(dir)) if dir.is_dir()
            ));

            assert!(store.untrust(sandbox.path()).unwrap());
            assert!(!store.is_trusted(&file));
        }

        #[test]
        fn trusts_configs_that_do_not_exist_yet() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/.gitkeep", "");

            let store = create_store(sandbox.path());
            store.trust(&sandbox.path().join("a")).unwrap();

            assert!(store.is_trusted(&sandbox.path().join("a/.prototools")));
        }

        #[test]
        fn untrusts() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let store = create_store(sandbox.path());
            let dir = sandbox.path().to_path_buf();

            store.trust(&dir).unwrap();

            assert!(store.untrust(&dir).unwrap());
            assert!(!store.untrust(&dir).unwrap());
            assert!(!store.is_trusted(&dir.join(".prototools")));
        }

        #[test]
        fn trusts_everything() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let mut store = create_store(sandbox.path());
            store.trust_all = true;

            assert_eq!(
                store.get_trust_source(&sandbox.path().join(".prototools")),
                Some(ProtoTrustSource::Ci)
            );
        }

        #[test]
        fn trusts_within_paths() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("trusted/nested/.prototools", "");
            sandbox.create_file("trusted-sibling/.prototools", "");

            let mut store = create_store(sandbox.path());
            store.add_trusted_path(&sandbox.path().join("trusted"));

            assert!(matches!(
                store.get_trust_source(&sandbox.path().join("trusted/nested/.prototools")),
                Some(ProtoTrustSource::TrustedPath(_))
            ));
            assert!(!store.is_trusted(&sandbox.path().join("trusted-sibling/.prototools")));
        }

        #[test]
        fn ignores_corrupt_records() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let store = create_store(sandbox.path());
            let dir = store.trust(sandbox.path()).unwrap();

            std::fs::write(store.get_record_path(&dir), "{").unwrap();

            assert!(!store.is_trusted(&dir.join(".prototools")));
        }
    }
}
