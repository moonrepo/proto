use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("{0}")]
    Message(String),

    #[error("Unable to install {tool}, unsupported architecture {arch}.")]
    UnsupportedArch { tool: String, arch: String },

    #[error("{tool} does not support canary versions or installations.")]
    UnsupportedCanary { tool: String },

    #[error("Unable to install {tool}, unsupported OS {os}.")]
    UnsupportedOS { tool: String, os: String },

    #[error("Unable to install {tool}, unsupported architecture {arch} for {os}.")]
    UnsupportedTarget {
        tool: String,
        arch: String,
        os: String,
    },
}
