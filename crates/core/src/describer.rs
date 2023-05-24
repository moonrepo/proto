#[async_trait::async_trait]
pub trait Describable<'tool>: Send + Sync {
    /// Return an identifier for the tool. Will also be used in variables,
    /// file names, and more.
    fn get_id(&self) -> &str;

    /// Return a human readable name of the tool.
    fn get_name(&self) -> String;
}
