use crate::config::{
    PROTO_PLUGIN_KEY, PartialProtoConfig, PartialProtoSettingsConfig, PartialProtoToolConfig,
};
use crate::config_error::ProtoConfigError;
use crate::id::Id;
use crate::tool_context::ToolContext;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use starbase_utils::json::{self, JsonError, JsonMap, JsonValue};
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
/// once the config has been trusted.
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

/// The security-sensitive settings of a config file.
#[derive(Clone, Debug, PartialEq)]
pub struct ProtoConfigSensitive {
    /// Paths to the settings, for display purposes.
    pub fields: Vec<String>,

    /// Hash of the settings, which a trust record must match.
    pub hash: String,
}

impl ProtoConfigSensitive {
    /// Extract the security-sensitive settings from a config, exactly as
    /// written in the file. Returns `None` if the config has none, in
    /// which case it does not need to be trusted.
    pub fn from_config(config: &PartialProtoConfig) -> Result<Option<Self>, ProtoConfigError> {
        let (_, sensitive) = split_config(config.to_owned());

        let value = json::serde_json::to_value(&sensitive).map_err(|error| {
            Box::new(JsonError::Format {
                error: Box::new(error),
            })
        })?;

        let Some(value) = canonicalize_json(value) else {
            return Ok(None);
        };

        Ok(Some(Self {
            fields: collect_fields(&value),
            hash: hash::sha256::from_bytes(json::format(&value, false).map_err(Box::new)?),
        }))
    }
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

/// Canonicalize a JSON value so that it hashes deterministically: object keys
/// are sorted, and nulls and empty objects (settings that were not configured)
/// are removed. Returns `None` if nothing remains.
fn canonicalize_json(value: JsonValue) -> Option<JsonValue> {
    match value {
        JsonValue::Null => None,
        JsonValue::Object(map) => {
            let mut entries = map
                .into_iter()
                .filter_map(|(key, value)| canonicalize_json(value).map(|value| (key, value)))
                .collect::<Vec<_>>();

            if entries.is_empty() {
                return None;
            }

            entries.sort_by(|a, b| a.0.cmp(&b.0));

            Some(JsonValue::Object(JsonMap::from_iter(entries)))
        }
        // Arrays are kept even when empty, as that is an explicit value
        JsonValue::Array(items) => Some(JsonValue::Array(
            items
                .into_iter()
                .map(|item| canonicalize_json(item).unwrap_or(JsonValue::Null))
                .collect(),
        )),
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

    fields
}

/// A record that the security-sensitive settings of a config file
/// have been trusted by the user.
#[derive(Deserialize, Serialize)]
struct ProtoTrustRecord {
    hash: String,
    path: PathBuf,
}

/// Determines which config files are trusted. A config is trusted when
/// the user has trusted its current security-sensitive settings (with
/// `proto trust`), when it's located within a trusted path, or when
/// running in CI.
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
        let mut trusted_paths = vec![];

        if let Some(value) = env::var_os("PROTO_TRUSTED_PATHS") {
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
            trust_all: ci_env::is_ci(),
            trusted_paths,
        }
    }

    /// Trust all config files within the provided directory.
    pub fn add_trusted_path(&mut self, dir: &Path) {
        self.trusted_paths.push(normalize_path(dir));
    }

    /// Return true if the provided config file, with the provided
    /// security-sensitive settings, has been trusted.
    pub fn is_trusted(&self, config_path: &Path, sensitive: &ProtoConfigSensitive) -> bool {
        if self.trust_all {
            trace!(config = ?config_path, "Trusting config as all configs are trusted (CI)");

            return true;
        }

        let config_path = normalize_path(config_path);

        if self.is_trusted_path(&config_path) {
            trace!(config = ?config_path, "Trusting config as it's within a trusted path");

            return true;
        }

        self.has_record(&config_path, &sensitive.hash)
    }

    /// Return true if the config file is trusted by the environment (CI or
    /// trusted paths), instead of an explicit record.
    pub fn is_implicitly_trusted(&self, config_path: &Path) -> bool {
        self.trust_all || self.is_trusted_path(&normalize_path(config_path))
    }

    /// Trust the security-sensitive settings of the provided config file,
    /// replacing any previous trust.
    pub fn trust(&self, config_path: &Path, hash: &str) -> Result<(), ProtoConfigError> {
        let config_path = normalize_path(config_path);

        debug!(config = ?config_path, hash, "Trusting config");

        json::write_file(
            self.get_record_path(&config_path),
            &ProtoTrustRecord {
                hash: hash.to_owned(),
                path: config_path,
            },
            true,
        )
        .map_err(Box::new)?;

        Ok(())
    }

    /// Remove trust for the provided config file. Returns true if the
    /// config was previously trusted.
    pub fn untrust(&self, config_path: &Path) -> Result<bool, ProtoConfigError> {
        let config_path = normalize_path(config_path);
        let record_path = self.get_record_path(&config_path);

        if !record_path.exists() {
            return Ok(false);
        }

        debug!(config = ?config_path, "Untrusting config");

        fs::remove_file(record_path)?;

        Ok(true)
    }

    fn has_record(&self, config_path: &Path, hash: &str) -> bool {
        let record_path = self.get_record_path(config_path);

        if !record_path.exists() {
            return false;
        }

        // Treat unreadable records as untrusted
        match json::read_file::<ProtoTrustRecord>(&record_path) {
            Ok(record) => record.path == config_path && record.hash == hash,
            Err(error) => {
                debug!(
                    record = ?record_path,
                    error = ?error,
                    "Failed to read trust record, treating config as untrusted"
                );

                false
            }
        }
    }

    // One file per config, so that trusting multiple configs concurrently
    // never has to read, modify, and write a shared file
    fn get_record_path(&self, config_path: &Path) -> PathBuf {
        self.dir.join(format!(
            "{}.json",
            hash::sha256::from_bytes(config_path.as_os_str().as_encoded_bytes())
        ))
    }

    fn is_trusted_path(&self, config_path: &Path) -> bool {
        self.trusted_paths
            .iter()
            .any(|dir| config_path.starts_with(dir))
    }
}

// Resolve symlinks so that a config is trusted regardless of the path used to reach it
fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
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

    fn sensitive(content: &str) -> Option<ProtoConfigSensitive> {
        ProtoConfigSensitive::from_config(&parse(content)).unwrap()
    }

    mod sensitive {
        use super::*;

        #[test]
        fn none_for_empty_config() {
            assert_eq!(sensitive(""), None);
        }

        #[test]
        fn none_for_safe_settings() {
            assert_eq!(
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
                ),
                None
            );
        }

        #[test]
        fn none_for_empty_tables() {
            assert_eq!(
                sensitive("[env]\n[settings]\n[shell]\n[tools.node]\n"),
                None
            );
        }

        #[test]
        fn includes_env_vars() {
            let data = sensitive("[env]\nBASH_ENV = \"script.sh\"\n").unwrap();

            assert_eq!(data.fields, ["env"]);
        }

        #[test]
        fn includes_env_files() {
            let data = sensitive("[env]\nfile = \".env\"\n").unwrap();

            assert_eq!(data.fields, ["env"]);
        }

        #[test]
        fn includes_shell_aliases() {
            let data = sensitive("[shell.aliases]\nls = \"echo\"\n").unwrap();

            assert_eq!(data.fields, ["shell.aliases"]);
        }

        #[test]
        fn includes_plugins() {
            let data = sensitive(
                r#"
[plugins.tools]
foo = "file://./foo.wasm"

[tools.bar]
plugin = "https://example.com/bar.wasm"
"#,
            )
            .unwrap();

            assert_eq!(data.fields, ["plugins.tools", "tools.bar"]);
        }

        #[test]
        fn includes_tool_and_backend_config() {
            let data = sensitive(
                r#"
[tools.node]
dist-url = "https://example.com"

[tools.node.env]
KEY = "value"

[backends.asdf]
repository = "https://example.com"
"#,
            )
            .unwrap();

            assert_eq!(data.fields, ["backends.asdf", "tools.node"]);
        }

        #[test]
        fn includes_unsafe_settings() {
            let data = sensitive(
                r#"
[settings]
auto-install = true
lockfile = true

[settings.url-rewrites]
"github.com" = "example.com"
"#,
            )
            .unwrap();

            assert_eq!(
                data.fields,
                ["settings.auto-install", "settings.url-rewrites"]
            );
        }

        #[test]
        fn includes_proto_pin() {
            let data = sensitive("node = \"20\"\nproto = \"0.40.0\"\n").unwrap();

            assert_eq!(data.fields, ["proto"]);
        }

        #[test]
        fn hash_ignores_safe_settings() {
            let a = sensitive("node = \"20\"\n[env]\nKEY = \"value\"\n").unwrap();
            let b = sensitive("node = \"22\"\nbun = \"1\"\n[env]\nKEY = \"value\"\n").unwrap();

            assert_eq!(a.hash, b.hash);
        }

        #[test]
        fn hash_ignores_formatting_and_order() {
            let a = sensitive("[env]\nA = \"a\"\n[shell.aliases]\nx = \"y\"\nz = \"w\"\n").unwrap();
            let b = sensitive(
                "[shell.aliases]\nz = \"w\"\n  x = \"y\"\n\n# comment\n[env]\nA = \"a\"\n",
            )
            .unwrap();

            assert_eq!(a.hash, b.hash);
        }

        #[test]
        fn hash_changes_with_sensitive_settings() {
            let a = sensitive("[env]\nKEY = \"value\"\n").unwrap();
            let b = sensitive("[env]\nKEY = \"other\"\n").unwrap();
            let c = sensitive("[env]\nKEY = \"value\"\nBASH_ENV = \"script.sh\"\n").unwrap();

            assert_ne!(a.hash, b.hash);
            assert_ne!(a.hash, c.hash);
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

        fn create_sensitive(hash: &str) -> ProtoConfigSensitive {
            ProtoConfigSensitive {
                fields: vec![],
                hash: hash.into(),
            }
        }

        #[test]
        fn not_trusted_by_default() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let store = create_store(sandbox.path());

            assert!(!store.is_trusted(&sandbox.path().join(".prototools"), &create_sensitive("a")));
        }

        #[test]
        fn trusts_matching_hash() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let store = create_store(sandbox.path());
            let path = sandbox.path().join(".prototools");

            store.trust(&path, "a").unwrap();

            assert!(store.is_trusted(&path, &create_sensitive("a")));
            assert!(!store.is_trusted(&path, &create_sensitive("b")));
        }

        #[test]
        fn trust_is_per_path() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a/.prototools", "");
            sandbox.create_file("b/.prototools", "");

            let store = create_store(sandbox.path());

            store
                .trust(&sandbox.path().join("a/.prototools"), "a")
                .unwrap();

            assert!(!store.is_trusted(
                &sandbox.path().join("b/.prototools"),
                &create_sensitive("a")
            ));
        }

        #[test]
        fn untrusts() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let store = create_store(sandbox.path());
            let path = sandbox.path().join(".prototools");

            store.trust(&path, "a").unwrap();

            assert!(store.untrust(&path).unwrap());
            assert!(!store.untrust(&path).unwrap());
            assert!(!store.is_trusted(&path, &create_sensitive("a")));
        }

        #[test]
        fn trusts_everything() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let mut store = create_store(sandbox.path());
            store.trust_all = true;

            assert!(store.is_trusted(&sandbox.path().join(".prototools"), &create_sensitive("a")));
        }

        #[test]
        fn trusts_within_paths() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("trusted/nested/.prototools", "");
            sandbox.create_file("trusted-sibling/.prototools", "");

            let mut store = create_store(sandbox.path());
            store.add_trusted_path(&sandbox.path().join("trusted"));

            assert!(store.is_trusted(
                &sandbox.path().join("trusted/nested/.prototools"),
                &create_sensitive("a")
            ));
            assert!(!store.is_trusted(
                &sandbox.path().join("trusted-sibling/.prototools"),
                &create_sensitive("a")
            ));
        }

        #[test]
        fn ignores_corrupt_records() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".prototools", "");

            let store = create_store(sandbox.path());
            let path = sandbox.path().join(".prototools");

            store.trust(&path, "a").unwrap();

            let record_path = store.get_record_path(&normalize_path(&path));
            std::fs::write(&record_path, "{").unwrap();

            assert!(!store.is_trusted(&path, &create_sensitive("a")));
        }
    }
}
