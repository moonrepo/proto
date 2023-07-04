use crate::PythonLanguage;
use proto_core::{
    async_trait, color, create_version_manifest_from_tags, ProtoError, Resolvable, Tool,
    VersionManifest,
};
use tokio::process::Command;

#[async_trait]
impl Resolvable<'_> for PythonLanguage {
    fn get_resolved_version(&self) -> &str {
        match self.version.as_ref() {
            Some(version) => version,
            None => "latest",
        }
    }

    async fn load_version_manifest(&self) -> Result<VersionManifest, ProtoError> {
        let output = match Command::new("rye")
            .args(["toolchain", "list", "--include-downloadable"])
            .output()
            .await
        {
            Ok(o) => o,
            Err(_) => {
                return Err(ProtoError::Message(format!(
            "proto requires {} to be installed and available on {} to use Python. Please install it and try again.",
            color::shell("rye"),
            color::id("PATH"),
        )));
            }
        };

        let available_versions = String::from_utf8_lossy(&output.stdout)
            .lines()
            .into_iter()
            .filter(|line| line.starts_with("cpython"))
            .map(|line| {
                let v = if let Some((version, _)) = line.split_once(" ") {
                    version
                } else {
                    line
                };
                return v.strip_prefix("cpython@").unwrap().to_owned();
            })
            .collect::<Vec<String>>();

        let mut manifest = create_version_manifest_from_tags(available_versions);
        manifest.inherit_aliases(&self.get_manifest()?.aliases);

        Ok(manifest)
    }

    fn set_version(&mut self, version: &str) {
        self.version = Some(version.to_owned());
    }
}
