use crate::PythonLanguage;
use proto_core::{async_trait, ProtoError, Shimable};

#[async_trait]
impl Shimable<'_> for PythonLanguage {
    // Don't create shims and rely on binaries found in `~/.rye`.
    async fn create_shims(&mut self, _find_only: bool) -> Result<(), ProtoError> {
        Ok(())
    }
}
