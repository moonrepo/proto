use proto_core::ProtoEnvironment;
use starbase_utils::fs;
use std::collections::HashMap;
use std::env::consts;

pub enum Metric {
    InstallTool,
    UninstallTool,
    UpgradeProto,
}

impl Metric {
    pub fn get_url(self) -> String {
        format!(
            "https://launch.moonrepo.app/{}",
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
    if !proto.load_config()?.settings.telemetry {
        return Ok(());
    }

    let mut client = reqwest::Client::new().post(metric.get_url());

    headers.insert("UID".into(), load_or_create_anonymous_uid(proto)?);
    headers.insert("CLI".into(), env!("CARGO_PKG_VERSION").to_owned());
    headers.insert("OS".into(), consts::OS.to_owned());
    headers.insert("Arch".into(), consts::ARCH.to_owned());

    for (key, value) in headers {
        client = client.header(format!("X-Proto-{key}"), value);
    }

    // Don't crash proto if the request fails for some reason
    if let Err(error) = client.send().await {
        dbg!(error);
    }

    Ok(())
}
