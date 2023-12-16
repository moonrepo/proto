use proto_core::{is_offline, ProtoEnvironment};
use starbase_utils::fs;
use std::collections::HashMap;
use std::env::consts;
use tracing::debug;

pub enum Metric {
    InstallTool,
    UninstallTool,
    UpgradeProto,
}

impl Metric {
    pub fn get_url(self) -> String {
        format!(
            "https://launch.moonrepo.app/{}",
            // "http://0.0.0.0:8081/{}",
            match self {
                Metric::InstallTool => "proto/install_tool",
                Metric::UninstallTool => "proto/uninstall_tool",
                Metric::UpgradeProto => "proto/upgrade_proto",
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

pub async fn track_usage(
    proto: &ProtoEnvironment,
    metric: Metric,
    mut headers: HashMap<String, String>,
) -> miette::Result<()> {
    let config = proto.load_config()?;

    if !config.settings.telemetry || is_offline() {
        return Ok(());
    }

    headers.insert("UID".into(), load_or_create_anonymous_uid(proto)?);
    headers.insert("CLI".into(), env!("CARGO_PKG_VERSION").to_owned());
    headers.insert("OS".into(), consts::OS.to_owned());
    headers.insert("Arch".into(), consts::ARCH.to_owned());

    let mut client = reqwest::Client::new().post(metric.get_url());

    for (key, value) in headers {
        client = client.header(format!("X-Proto-{key}"), value);
    }

    // Don't crash proto if the request fails for some reason
    if let Err(error) = client.send().await {
        debug!("Failed to track usage metric: {}", error.to_string());
    }

    Ok(())
}
