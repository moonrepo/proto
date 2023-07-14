use std::path::PathBuf;

pub struct GitHubLocator {
    pub asset_name: String, // Without extension
    pub repo_slug: String,
    pub version: Option<String>,
}

pub struct WapmLocator {
    pub package_name: String,
    pub version: Option<String>,
    pub wasm_name: String, // Without .wasm
}

pub enum PluginLocator {
    // source:path/to/file.wasm
    SourceFile { file: String, path: PathBuf },

    // source:https://url/to/file.wasm
    SourceUrl { url: String },

    // github:owner/repo@version
    GitHub(GitHubLocator),

    // wapm:package/name@version
    Wapm(WapmLocator),
}
