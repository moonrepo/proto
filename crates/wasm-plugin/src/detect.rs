use crate::WasmPlugin;
use proto_core::{async_trait, load_version_file, Detector, ProtoError};
use proto_pdk_api::{DetectVersionOutput, ParseVersionFileInput, ParseVersionFileOutput};
use starbase_utils::fs;
use std::path::Path;

#[async_trait]
impl Detector<'_> for WasmPlugin {
    async fn detect_version_from(&self, working_dir: &Path) -> Result<Option<String>, ProtoError> {
        if !self.container.has_func("detect_version_files") {
            return Ok(None);
        }

        let has_parser = self.container.has_func("parse_version_file");
        let result: DetectVersionOutput = self
            .container
            .cache_func("detect_version_files")
            .map_err(|e| ProtoError::Message(e.to_string()))?;

        for file in result.files {
            let file_path = working_dir.join(&file);

            if !file_path.exists() {
                continue;
            }

            if has_parser {
                let result: ParseVersionFileOutput = self
                    .container
                    .call_func_with(
                        "parse_version_file",
                        ParseVersionFileInput {
                            content: fs::read_file(&file_path)?,
                            env: self.get_environment()?,
                            file: file.clone(),
                        },
                    )
                    .map_err(|e| ProtoError::Message(e.to_string()))?;

                if result.version.is_none() {
                    continue;
                }

                return Ok(result.version);
            }

            return Ok(Some(load_version_file(&file_path)?));
        }

        Ok(None)
    }
}
