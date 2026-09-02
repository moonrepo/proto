use proto_core::{
    PluginLocator, PluginType, ProtoEnvironment, ProtoLoaderError, ToolContext, load_schema_config,
    locate_plugin,
};
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use starbase_utils::json::json;

mod load_schema_config {
    use super::*;

    fn get_expected() -> serde_json::Value {
        json!({
          "install": {
            "arch": {
              "x86_64": "x64",
              "aarch64": "arm64"
            },
            "exes": {
              "fooBar": {
                "exe-path": "./file",
                "primary": true,
                "shim-env-vars": {
                  "FOO_BAR": "value"
                }
              }
            }
          },
          "platform": {
            "linux": {
              "archive-prefix": "package",
              "download-file": "linux.tgz"
            }
          },
          "resolve": {
            "aliases": {
              "fooBar": "1.2.3",
              "next1": "4.5.6"
            }
          }
        })
    }

    #[test]
    fn convert_keys_for_json_files() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            "schema.json",
            r#"{
  "install": {
    "arch": {
      "x86_64": "x64",
      "aarch64": "arm64"
    },
    "exes": {
      "fooBar": {
        "exePath": "./file",
        "primary": true,
        "shimEnvVars": {
          "FOO_BAR": "value"
        }
      }
    }
  },
  "platform": {
    "linux": {
      "archivePrefix": "package",
      "downloadFile": "linux.tgz"
    }
  },
  "resolve": {
    "aliases": {
      "fooBar": "1.2.3",
      "next1": "4.5.6"
    }
  }
}"#,
        );

        let value = load_schema_config(&sandbox.path().join("schema.json")).unwrap();

        assert_eq!(value, get_expected());
    }

    #[test]
    fn convert_keys_for_yaml_files() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            "schema.yaml",
            r#"
install:
  arch:
    x86_64: x64
    aarch64: arm64
  exes:
    fooBar:
      exePath: './file'
      primary: true
      shimEnvVars:
        FOO_BAR: value
platform:
  linux:
    archivePrefix: package
    downloadFile: linux.tgz
resolve:
  aliases:
    fooBar: '1.2.3'
    next1: '4.5.6'
"#,
        );

        let value = load_schema_config(&sandbox.path().join("schema.yaml")).unwrap();

        assert_eq!(value, get_expected());
    }

    #[test]
    fn doesnt_convert_keys_for_toml_files() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            "schema.toml",
            r#"
[install]
arch = { "x86_64" = "x64", "aarch64" = "arm64" }

[install.exes.fooBar]
exe-path = "./file"
primary = true
shim-env-vars = { "FOO_BAR" = "value" }

[platform.linux]
archive-prefix = "package"
download-file = "linux.tgz"

[resolve.aliases]
fooBar = "1.2.3"
next1 = "4.5.6"
"#,
        );

        let value = load_schema_config(&sandbox.path().join("schema.toml")).unwrap();

        assert_eq!(value, get_expected());
    }
}

mod locate_plugin {
    use super::*;

    // Build a sandbox with a `.prototools` and a ProtoEnvironment whose
    // working_dir is the sandbox, so config lookups read the fixture file.
    fn setup(prototools: &str) -> (Sandbox, ProtoEnvironment) {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".prototools", prototools);

        let mut proto = ProtoEnvironment::new_testing(sandbox.path()).unwrap();
        proto.working_dir = sandbox.path().to_path_buf();

        write_community_cache(&proto, "[]");

        (sandbox, proto)
    }

    fn write_community_cache(proto: &ProtoEnvironment, contents: &str) {
        let path = proto
            .store
            .cache_dir
            .join("registry/community-plugins.json");

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn write_act_community_cache(proto: &ProtoEnvironment) {
        write_community_cache(
            proto,
            r#"[
  {
    "id": "act",
    "locator": "https://raw.githubusercontent.com/moonrepo/community-plugins/master/tools/act.toml",
    "format": "toml",
    "name": "act",
    "description": "Run your GitHub Actions locally.",
    "author": "moonrepo",
    "bins": ["act"]
  }
]"#,
        );
    }

    fn ctx(value: &str) -> ToolContext {
        ToolContext::parse(value).unwrap()
    }

    // User-defined plugins in `[plugins.tools]` must beat the new
    // registry-fallback branch even when a default registry is configured.
    #[tokio::test]
    async fn user_locator_takes_priority_over_registry() {
        let (_sandbox, proto) = setup(
            r#"
[plugins.tools]
foo = "github://moonrepo/foo"

[[settings.registries]]
registry = "ghcr.io"
namespace = "moonrepo"
default = true
"#,
        );

        let result = locate_plugin(&ctx("foo"), &proto, PluginType::Tool)
            .await
            .unwrap();

        let PluginLocator::GitHub(github) = result else {
            panic!("expected GitHub locator, got {result:?}");
        };

        assert_eq!(github.repo_slug, "moonrepo/foo");
    }

    // Built-in plugins resolve through `builtin_plugins()` before reaching
    // the registry fallback. With no debug WASM available, the synthesized
    // locator is a Registry pointing at the moonrepo namespace.
    #[tokio::test]
    async fn builtin_plugin_resolves_via_builtins() {
        let (_sandbox, proto) = setup("");

        let result = locate_plugin(&ctx("node"), &proto, PluginType::Tool)
            .await
            .unwrap();

        match result {
            // Local debug builds may resolve to a File locator pointing at a
            // pre-built WASM artifact; CI/release builds synthesize a Registry
            // locator. Both indicate the builtin branch fired.
            PluginLocator::Registry(registry) => {
                assert_eq!(registry.image, "node_tool");
            }
            PluginLocator::File(_) => {}
            other => panic!("expected Registry or File locator, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn community_plugin_resolves_via_remote_registry() {
        let (_sandbox, proto) = setup(
            r#"
[[settings.registries]]
registry = "ghcr.io"
namespace = "fallback"
default = true
"#,
        );

        write_act_community_cache(&proto);

        let result = locate_plugin(&ctx("act"), &proto, PluginType::Tool)
            .await
            .unwrap();

        assert_eq!(
            result.to_string(),
            "https://raw.githubusercontent.com/moonrepo/community-plugins/master/tools/act.toml"
        );
    }

    #[tokio::test]
    async fn community_registry_instance_is_reused_across_environment_clones() {
        let (_sandbox, proto) = setup("");
        let cloned_proto = proto.clone();
        write_act_community_cache(&proto);

        let first = locate_plugin(&ctx("act"), &proto, PluginType::Tool)
            .await
            .unwrap();

        write_community_cache(&proto, "[]");

        let second = locate_plugin(&ctx("act"), &cloned_proto, PluginType::Tool)
            .await
            .unwrap();

        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn community_registry_can_be_disabled() {
        let (_sandbox, proto) = setup(
            r#"
[settings]
community-tools = false
"#,
        );
        write_act_community_cache(&proto);

        let error = locate_plugin(&ctx("act"), &proto, PluginType::Tool)
            .await
            .expect_err("disabled community tools should not resolve");

        assert!(matches!(error, ProtoLoaderError::UnknownTool { .. }));
    }

    #[tokio::test]
    async fn user_locator_takes_priority_over_community_plugin() {
        let (_sandbox, proto) = setup(
            r#"
[plugins.tools]
act = "github://example/act-plugin"
"#,
        );

        let result = locate_plugin(&ctx("act"), &proto, PluginType::Tool)
            .await
            .unwrap();

        assert_eq!(result.to_string(), "github://example/act-plugin");
    }

    #[tokio::test]
    async fn community_registry_does_not_resolve_backends() {
        let (_sandbox, proto) = setup("");

        let error = locate_plugin(&ctx("act:example"), &proto, PluginType::Backend)
            .await
            .expect_err("community plugins should only resolve tools");

        assert!(matches!(error, ProtoLoaderError::UnknownTool { .. }));
    }

    // The central new-behavior test: with a `default = true` registry,
    // an unknown id resolves to a Registry locator built from
    // `registry.get_reference(id)` (so the namespace is preserved).
    #[tokio::test]
    async fn unknown_id_with_default_registry_returns_registry_locator() {
        let (_sandbox, proto) = setup(
            r#"
[[settings.registries]]
registry = "ghcr.io"
namespace = "moonrepo"
default = true
"#,
        );

        let result = locate_plugin(&ctx("foo"), &proto, PluginType::Tool)
            .await
            .unwrap();

        let PluginLocator::Registry(registry) = result else {
            panic!("expected Registry locator, got {result:?}");
        };

        assert_eq!(registry.registry, Some("ghcr.io".into()));
        assert_eq!(registry.namespace, Some("moonrepo".into()));
        assert_eq!(registry.image, "foo");
        assert!(registry.tag.is_none());
    }

    // Regression guard for the GHCR migration: the auto-default registry
    // ships with `default: false`, so an unknown id must error rather than
    // silently resolve to ghcr.io.
    #[tokio::test]
    async fn unknown_id_without_default_registry_returns_unknown_tool_error() {
        let (_sandbox, proto) = setup("");

        let err = locate_plugin(&ctx("totally_unknown_xyz"), &proto, PluginType::Tool)
            .await
            .expect_err("expected UnknownTool error");

        match err {
            ProtoLoaderError::UnknownTool { context } => {
                assert_eq!(context, "totally_unknown_xyz");
            }
            other => panic!("expected UnknownTool, got {other:?}"),
        }
    }

    // Multiple registries: only the one marked `default = true` is selected,
    // regardless of order in the list.
    #[tokio::test]
    async fn multiple_registries_only_default_is_selected() {
        let (_sandbox, proto) = setup(
            r#"
[[settings.registries]]
registry = "registry.example.com"
namespace = "first"
default = false

[[settings.registries]]
registry = "ghcr.io"
namespace = "second"
default = true

[[settings.registries]]
registry = "third.example.com"
namespace = "third"
default = false
"#,
        );

        let result = locate_plugin(&ctx("foo"), &proto, PluginType::Tool)
            .await
            .unwrap();

        let PluginLocator::Registry(registry) = result else {
            panic!("expected Registry locator, got {result:?}");
        };

        assert_eq!(registry.registry, Some("ghcr.io".into()));
        assert_eq!(registry.namespace, Some("second".into()));
        assert_eq!(registry.image, "foo");
    }

    // PluginType::Backend resolves the context's backend id against
    // `[plugins.backends]`, not `[plugins.tools]`.
    #[tokio::test]
    async fn backend_type_uses_backends_table() {
        let (_sandbox, proto) = setup(
            r#"
[plugins.backends]
asdf = "github://moonrepo/asdf-backend"

[plugins.tools]
asdf = "github://wrong/wrong"
"#,
        );

        let result = locate_plugin(&ctx("asdf:node"), &proto, PluginType::Backend)
            .await
            .unwrap();

        let PluginLocator::GitHub(github) = result else {
            panic!("expected GitHub locator, got {result:?}");
        };

        assert_eq!(github.repo_slug, "moonrepo/asdf-backend");
    }

    // A default registry with no namespace produces a locator with
    // `namespace == None` and `image == id`.
    #[tokio::test]
    async fn unknown_id_default_registry_no_namespace() {
        let (_sandbox, proto) = setup(
            r#"
[[settings.registries]]
registry = "registry.example.com"
default = true
"#,
        );

        let result = locate_plugin(&ctx("foo"), &proto, PluginType::Tool)
            .await
            .unwrap();

        let PluginLocator::Registry(registry) = result else {
            panic!("expected Registry locator, got {result:?}");
        };

        assert_eq!(registry.registry, Some("registry.example.com".into()));
        assert!(registry.namespace.is_none());
        assert_eq!(registry.image, "foo");
    }
}
