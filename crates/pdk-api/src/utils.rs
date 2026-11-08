use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

fn extract_version_from_line(line: &str) -> Option<&str> {
    let line = line.split('#').next().unwrap_or_default();
    let line = line.split("//").next().unwrap_or_default().trim();

    (!line.is_empty()).then_some(line)
}

/// Extract the version from a plain version file, like `.nvmrc` or `.python-version`:
/// the first line with content once `#` and `//` comments (which may take up the
/// whole line, or follow the version, as version managers allow) and surrounding
/// whitespace are removed.
pub fn extract_version_from_text(content: &str) -> Option<&str> {
    content.lines().find_map(extract_version_from_line)
}

/// Extract the version from the plain version file at the provided path, using
/// the same rules as [`extract_version_from_text`]. Lines are read one at a time,
/// and reading stops once a version has been found.
pub fn extract_version_from_file(path: impl AsRef<Path>) -> io::Result<Option<String>> {
    for line in BufReader::new(File::open(path)?).lines() {
        if let Some(version) = extract_version_from_line(&line?) {
            return Ok(Some(version.to_owned()));
        }
    }

    Ok(None)
}
