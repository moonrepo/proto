use proto_core::load_schema_config;
use starbase_sandbox::create_empty_sandbox;
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
