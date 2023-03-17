use crate::NodeLanguage;
use proto_core::{
    async_trait, Describable, Executable, Installable, ProtoError, Resolvable, ShimBuilder,
    Shimable,
};

fn npx_template() -> String {
    r#"# npx comes bundled with node, so first determine the node path...
node_bin=$(proto bin node)

# ...and then replace the node bin with npx. Simple but works!
npx_bin=$(echo "$node_bin" | sed 's/bin\/node/bin\/npx/')

exec "$npx_bin" -- "$@""#
        .to_owned()
}

#[async_trait]
impl Shimable<'_> for NodeLanguage {
    async fn create_shims(&mut self) -> Result<(), ProtoError> {
        let mut shimmer = ShimBuilder::new(self.get_bin_name(), self.get_bin_path()?);

        shimmer
            .dir(self.get_install_dir()?)
            .version(self.get_resolved_version());

        shimmer.create_global_shim()?;

        // npx
        let mut shimmer =
            ShimBuilder::new("npx", &self.get_bin_path()?.parent().unwrap().join("npx"));

        shimmer
            .dir(self.get_install_dir()?)
            .version(self.get_resolved_version())
            .set_global_template(npx_template());

        shimmer.create_global_shim()?;

        Ok(())
    }
}
