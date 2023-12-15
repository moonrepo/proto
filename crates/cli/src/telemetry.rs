use proto_core::ProtoEnvironment;
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

pub fn track_usage(
    proto: &ProtoEnvironment,
    metric: Metric,
    mut headers: HashMap<String, String>,
) -> miette::Result<()> {
    if !proto.load_config()?.settings.telemetry {
        return Ok(());
    }

    let mut client = reqwest::Client::new().post(metric.get_url());

    headers.insert("OS".into(), consts::OS.to_owned());
    headers.insert("Arch".into(), consts::ARCH.to_owned());

    for (key, value) in headers {
        client = client.header(format!("X-Proto-{key}"), value);
    }

    // Run in the background as we don't care if the request finishes or fails
    tokio::spawn(async {
        let _ = client.send().await;
    });

    Ok(())
}
