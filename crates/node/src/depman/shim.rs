use crate::depman::NodeDependencyManagerType;
use crate::NodeDependencyManager;
use proto_core::{
    async_trait, Executable, Installable, ProtoError, Resolvable, ShimBuilder, Shimable,
};
use std::path::Path;

#[cfg(not(windows))]
fn node_gyp_template() -> String {
    r#"# node-gyp comes bundled with npm, so first determine the npm path...
npm_bin=$(proto bin npm)

# ...and then replace the npm bin with node-gyp. Simple but works!
node_gyp_bin=$(echo "$npm_bin" | sed 's/npm-cli.js/node-gyp-bin\/node-gyp/')

exec "$node_gyp_bin" "$@""#
        .to_owned()
}

#[cfg(windows)]
fn node_gyp_template() -> String {
    r#"# node-gyp comes bundled with npm, so first determine the npm path...
$NpmBin = proto.exe bin npm

# ...and then replace the npm bin with node-gyp. Simple but works!
$NodeGypBin = $NpmBin.replace("npm-cli.js", "node-gyp-bin\\node-gyp.cmd")

& $NodeGypBin $args"#
        .to_owned()
}

#[async_trait]
impl Shimable<'_> for NodeDependencyManager {
    async fn create_shims(&mut self, find_only: bool) -> Result<(), ProtoError> {
        let mut shimmer = ShimBuilder::new(&self.package_name, self.get_bin_path()?)?;

        shimmer
            .dir(self.get_install_dir()?)
            .version(self.get_resolved_version())
            .parent("node");

        shimmer.create_global_shim()?;

        self.shim_path = Some(shimmer.create_tool_shim(find_only)?);

        // node-gyp
        if matches!(self.type_of, NodeDependencyManagerType::Npm) {
            let mut shimmer = ShimBuilder::new(
                "node-gyp",
                &self
                    .get_bin_path()?
                    .parent()
                    .unwrap()
                    .join("node-gyp-bin/node-gyp"),
            )?;

            shimmer
                .dir(self.get_install_dir()?)
                .version(self.get_resolved_version())
                .set_global_template(node_gyp_template());

            shimmer.create_global_shim()?;
        }

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.shim_path.as_deref()
    }
}
