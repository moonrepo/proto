// Windows generates different snapshots
#[cfg(not(windows))]
mod shimmer {
    use proto_core::{ProtoEnvironment, ShimContext};
    use starbase_sandbox::{assert_snapshot, create_empty_sandbox};
    use std::fs;
    use std::path::Path;

    fn create_context<'l>(id: &'l str, proto: &'l ProtoEnvironment) -> ShimContext<'l> {
        ShimContext {
            bin: id,
            tool_id: id,
            tool_dir: Some(proto.tools_dir.join(id).join("1.2.3")),
            tool_version: Some("1.2.3".into()),
            ..ShimContext::default()
        }
    }

    fn read_shim(shim: &Path, root: &Path) -> String {
        fs::read_to_string(shim)
            .unwrap()
            .replace(root.to_str().unwrap(), "")
    }

    #[test]
    fn doesnt_update_global_if_find_only() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".proto/shims/primary", "test");

        let proto = ProtoEnvironment::new_testing(sandbox.path());
        let context = create_context("primary", &proto);
        let shim_path = proto.shims_dir.join("primary");

        context.create_shim(&shim_path, true).unwrap();

        assert_eq!(read_shim(&shim_path, sandbox.path()), "test");
    }

    #[test]
    fn global() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());
        let context = create_context("primary", &proto);

        let shim_path = proto.shims_dir.join("primary");
        context.create_shim(&shim_path, false).unwrap();

        assert_snapshot!(read_shim(&shim_path, sandbox.path()));
    }

    #[test]
    fn global_with_args() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());

        let mut context = create_context("primary", &proto);
        context.before_args = Some("--a -b");
        context.after_args = Some("./file");

        let shim_path = proto.shims_dir.join("primary");
        context.create_shim(&shim_path, false).unwrap();

        assert_snapshot!(read_shim(&shim_path, sandbox.path()));
    }

    #[test]
    fn alt_global() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());

        let bin_path = "other/bin/path";
        let mut context = create_context("primary", &proto);
        context.alt_bin = Some(bin_path);

        let shim_path = proto.shims_dir.join("secondary");
        context.create_shim(&shim_path, false).unwrap();

        assert_snapshot!(read_shim(&shim_path, sandbox.path()));
    }

    #[test]
    fn alt_global_with_args() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());

        let bin_path = "other/bin/path";
        let mut context = create_context("primary", &proto);
        context.alt_bin = Some(bin_path);
        context.before_args = Some("--a -b");
        context.after_args = Some("./file");

        let shim_path = proto.shims_dir.join("secondary");
        context.create_shim(&shim_path, false).unwrap();

        assert_snapshot!(read_shim(&shim_path, sandbox.path()));
    }
}
