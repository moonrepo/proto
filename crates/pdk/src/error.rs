use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Unable to install {0}, unsupported architecture {1}.")]
    UnsupportedArchitecture(String, String),

    #[error("Unable to install {0}, unsupported platform {1}.")]
    UnsupportedPlatform(String, String),
}
