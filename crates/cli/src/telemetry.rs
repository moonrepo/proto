use proto_core::{is_offline, ProtoEnvironment};
use starbase_utils::fs;
use std::collections::HashMap;
use std::env::{self, consts};
use tracing::debug;

pub enum Metric {
    InstallTool {
        id: String,
        pinned: bool,
        plugin: String,
        version: String,
        version_candidate: String,
    },
    UninstallTool {
        id: String,
        plugin: String,
        version: String,
    },
    UpgradeProto {
        old_version: String,
        new_version: String,
    },
}

impl Metric {
    pub fn into_headers(self) -> HashMap<String, String> {
        match self {
            Metric::InstallTool {
                id,
                version,
                version_candidate,
                pinned,
                plugin,
            } => HashMap::from_iter([
                ("ToolId".into(), id),
                ("ToolPinned".into(), pinned.to_string()),
                ("ToolPlugin".into(), plugin),
                ("ToolVersion".into(), version),
                ("ToolVersionCandidate".into(), version_candidate),
            ]),
            Metric::UninstallTool {
                id,
                plugin,
                version,
            } => HashMap::from_iter([
                ("ToolId".into(), id),
                ("ToolPlugin".into(), plugin),
                ("ToolVersion".into(), version),
            ]),
            Metric::UpgradeProto {
                old_version,
                new_version,
            } => HashMap::from_iter([
                ("OldVersion".into(), old_version),
                ("NewVersion".into(), new_version),
            ]),
        }
    }

    pub fn get_url(&self) -> String {
        format!(
            "https://launch.moonrepo.app/{}",
            // "http://0.0.0.0:8081/{}",
            match self {
                Metric::InstallTool { .. } => "proto/install_tool",
                Metric::UninstallTool { .. } => "proto/uninstall_tool",
                Metric::UpgradeProto { .. } => "proto/upgrade_proto",
            }
        )
    }
}

fn load_or_create_anonymous_uid(proto: &ProtoEnvironment) -> miette::Result<String> {
    let id_path = proto.root.join("id");

    if id_path.exists() {
        return Ok(fs::read_file(id_path)?);
    }

    let id = uuid::Uuid::new_v4().to_string();

    fs::write_file(id_path, &id)?;

    Ok(id)
}

pub async fn track_usage(proto: &ProtoEnvironment, metric: Metric) -> miette::Result<()> {
    let config = proto.load_config()?;

    if !config.settings.telemetry || is_offline() || env::var("PROTO_TEST").is_ok() {
        return Ok(());
    }

    let mut client = reqwest::Client::new().post(metric.get_url());

    let mut headers = metric.into_headers();
    headers.insert("UID".into(), load_or_create_anonymous_uid(proto)?);
    headers.insert("CLI".into(), env!("CARGO_PKG_VERSION").to_owned());
    headers.insert("OS".into(), consts::OS.to_owned());
    headers.insert("Arch".into(), consts::ARCH.to_owned());
    headers.insert("CI".into(), env::var("CI").is_ok().to_string());

    for (key, value) in headers {
        client = client.header(format!("X-Proto-{key}"), value);
    }

    // Don't crash proto if the request fails for some reason
    if let Err(error) = client.send().await {
        debug!("Failed to track usage metric: {}", error.to_string());
    }

    Ok(())
}
