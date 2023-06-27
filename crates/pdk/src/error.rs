use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Unable to install {tool}, unsupported architecture {arch}.")]
    UnsupportedArchitecture { tool: String, arch: String },

    #[error("Unable to install {tool}, unsupported platform {platform}.")]
    UnsupportedPlatform { tool: String, platform: String },
}
