pub use starbase_utils::net::OnChunkFn;

#[derive(Default)]
pub enum InstallStrategy {
    BuildFromSource,
    #[default]
    DownloadPrebuilt,
}

pub enum InstallPhase {
    // Download -> verify -> unpack
    Download,
    Verify,
    Unpack,
}

pub type OnPhaseFn = Box<dyn Fn(InstallPhase) + Send>;

#[derive(Default)]
pub struct InstallOptions {
    pub on_download_chunk: Option<OnChunkFn>,
    pub on_phase_change: Option<OnPhaseFn>,
    pub strategy: InstallStrategy,
}
