use crate::GoLanguage;
use proto_core::{async_trait, Detector, ProtoError};
use starbase_utils::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

static GOPREFIX: &str = "go ";

#[async_trait]
impl Detector<'_> for GoLanguage {
    async fn detect_version_from(&self, working_dir: &Path) -> Result<Option<String>, ProtoError> {
        let gowork = working_dir.join("go.work");

        if gowork.exists() {
            if let Some(version) = scan_for_go_version(&gowork) {
                return Ok(Some(version));
            }
        }

        let gomod = working_dir.join("go.mod");

        if gomod.exists() {
            if let Some(version) = scan_for_go_version(&gomod) {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }
}

fn scan_for_go_version(path: &Path) -> Option<String> {
    if let Ok(file) = fs::open_file(path) {
        let buffered = BufReader::new(file);

        for line in buffered.lines().flatten() {
            if let Some(version) = line.strip_prefix(GOPREFIX) {
                return Some(version.into());
            }
        }
    }

    None
}
