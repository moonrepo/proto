// Windows generates different snapshots
#[cfg(not(windows))]
mod shimmer {
    use proto_core::{create_global_shim, create_local_shim, ProtoEnvironment, ShimContext};
    use starbase_sandbox::{assert_snapshot, create_empty_sandbox};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn create_context<'l>(id: &'l str, proto: &'l ProtoEnvironment) -> ShimContext<'l> {
        ShimContext {
            shim_file: id,
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
        let shim = create_global_shim(&proto, context, true).unwrap();

        assert_eq!(shim, proto.shims_dir.join("primary"));
        assert_eq!(read_shim(&shim, sandbox.path()), "test");
    }

    #[test]
    fn global() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());
        let context = create_context("primary", &proto);
        let shim = create_global_shim(&proto, &context, false).unwrap();

        assert_eq!(shim, proto.shims_dir.join("primary"));
        assert_snapshot!(read_shim(&shim, sandbox.path()));
    }

    #[test]
    fn global_with_args() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());

        let mut context = create_context("primary", &proto);
        context.before_args = Some("--a -b");
        context.after_args = Some("./file");

        let shim = create_global_shim(&proto, &context, false).unwrap();

        assert_eq!(shim, proto.shims_dir.join("primary"));
        assert_snapshot!(read_shim(&shim, sandbox.path()));
    }

    #[test]
    fn alt_global() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());

        let bin_path = PathBuf::from("other/bin/path");
        let mut context = create_context("primary", &proto);
        context.shim_file = "secondary";
        context.bin_path = Some(&bin_path);

        let shim = create_global_shim(&proto, &context, false).unwrap();

        assert_eq!(shim, proto.shims_dir.join("secondary"));
        assert_snapshot!(read_shim(&shim, sandbox.path()));
    }

    #[test]
    fn alt_global_with_args() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());

        let bin_path = PathBuf::from("other/bin/path");
        let mut context = create_context("primary", &proto);
        context.shim_file = "secondary";
        context.bin_path = Some(&bin_path);
        context.before_args = Some("--a -b");
        context.after_args = Some("./file");

        let shim = create_global_shim(&proto, &context, false).unwrap();

        assert_eq!(shim, proto.shims_dir.join("secondary"));
        assert_snapshot!(read_shim(&shim, sandbox.path()));
    }

    #[test]
    fn doesnt_update_local_if_find_only() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".proto/tools/tool/1.2.3/shims/tool", "test");

        let proto = ProtoEnvironment::new_testing(sandbox.path());
        let bin_path = PathBuf::from("bin/tool");
        let mut context = create_context("tool", &proto);
        context.bin_path = Some(&bin_path);

        let shim = create_local_shim(&context, true).unwrap();

        assert_eq!(shim, context.tool_dir.unwrap().join("shims/tool"));
        assert_eq!(read_shim(&shim, sandbox.path()), "test");
    }

    #[test]
    fn local() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());

        let bin_path = PathBuf::from("bin/tool");
        let mut context = create_context("tool", &proto);
        context.bin_path = Some(&bin_path);

        let shim = create_local_shim(&context, false).unwrap();

        assert_eq!(shim, context.tool_dir.unwrap().join("shims/tool"));
        assert_snapshot!(read_shim(&shim, sandbox.path()));
    }

    #[test]
    fn local_with_args() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());

        let bin_path = PathBuf::from("bin/tool");
        let mut context = create_context("tool", &proto);
        context.bin_path = Some(&bin_path);
        context.before_args = Some("--a -b");
        context.after_args = Some("./file");

        let shim = create_local_shim(&context, false).unwrap();

        assert_eq!(shim, context.tool_dir.unwrap().join("shims/tool"));
        assert_snapshot!(read_shim(&shim, sandbox.path()));
    }

    #[test]
    fn local_with_parent() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());

        let bin_path = PathBuf::from("bin/tool");
        let mut context = create_context("tool", &proto);
        context.bin_path = Some(&bin_path);
        context.parent_bin = Some("node");

        let shim = create_local_shim(&context, false).unwrap();

        assert_eq!(shim, context.tool_dir.unwrap().join("shims/tool"));
        assert_snapshot!(read_shim(&shim, sandbox.path()));
    }

    #[test]
    fn local_with_parent_and_args() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());

        let bin_path = PathBuf::from("bin/tool");
        let mut context = create_context("tool", &proto);
        context.bin_path = Some(&bin_path);
        context.parent_bin = Some("node");
        context.before_args = Some("--a -b");
        context.after_args = Some("./file");

        let shim = create_local_shim(&context, false).unwrap();

        assert_eq!(shim, context.tool_dir.unwrap().join("shims/tool"));
        assert_snapshot!(read_shim(&shim, sandbox.path()));
    }

    #[test]
    fn tool_id_formats() {
        let sandbox = create_empty_sandbox();
        let proto = ProtoEnvironment::new_testing(sandbox.path());

        let bin_path = PathBuf::from("bin/tool");
        let mut context = create_context("tool-name", &proto);
        context.bin_path = Some(&bin_path);
        context.parent_bin = Some("parentName");

        let shim = create_local_shim(&context, false).unwrap();

        assert_eq!(shim, context.tool_dir.unwrap().join("shims/tool-name"));
        assert_snapshot!(read_shim(&shim, sandbox.path()));
    }
}
