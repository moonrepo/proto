use proto_core::{
    Detector, Downloadable, Executable, Installable, Proto, Resolvable, Shimable, Tool, Verifiable,
};
use proto_wasm_plugin::WasmPlugin;
use starbase_sandbox::create_empty_sandbox;
use std::env::{self, consts};
use std::fs;
use std::path::{Path, PathBuf};

static mut LOGGING: bool = false;

fn create_plugin(dir: &Path) -> WasmPlugin {
    let wasm_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32-wasi/debug")
        .canonicalize()
        .unwrap();

    unsafe {
        if !LOGGING {
            LOGGING = true;

            extism::set_log_file(wasm_dir.join("proto_wasm_test.log"), Some(log::Level::Info));
        }
    };

    let mut tool = WasmPlugin::new(
        Proto::from(dir),
        "wasm".into(),
        wasm_dir.join("proto_wasm_test.wasm"),
    )
    .unwrap();

    // Node.js version, so we can test downloading
    tool.version = Some("20.0.0".into());
    tool
}

fn create_plugin_without_version(dir: &Path) -> WasmPlugin {
    let mut tool = create_plugin(dir);
    tool.version = None;
    tool
}

fn get_arch() -> String {
    match consts::ARCH {
        "aarch64" => "arm64".into(),
        "x86_64" => "x64".into(),
        "x86" => "x86".into(),
        other => other.into(),
    }
}

fn get_file() -> String {
    let arch = get_arch();

    match consts::OS {
        "linux" => format!("node-v20.0.0-linux-{arch}.tar.xz"),
        "macos" => format!("node-v20.0.0-darwin-{arch}.tar.xz"),
        "windows" => format!("node-v20.0.0-win-{arch}.zip"),
        _ => unimplemented!(),
    }
}

mod wasm_plugin {
    use super::*;

    #[tokio::test]
    async fn downloads_verifies_installs_tool() {
        let fixture = create_empty_sandbox();
        let mut tool = create_plugin(fixture.path());

        env::set_var("PROTO_ROOT", fixture.path().to_string_lossy().to_string());

        tool.setup("20.0.0").await.unwrap();

        env::remove_var("PROTO_ROOT");

        let bin = tool.get_bin_path().unwrap();

        if cfg!(windows) {
            assert_eq!(bin, tool.get_install_dir().unwrap().join("node.exe"));
        } else {
            assert_eq!(bin, tool.get_install_dir().unwrap().join("bin/node"));
        }
    }

    mod detector {
        use super::*;

        #[tokio::test]
        async fn doesnt_match_if_no_files() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            assert_eq!(
                tool.detect_version_from(fixture.path()).await.unwrap(),
                None
            );
        }

        #[tokio::test]
        async fn matches_standard() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            fixture.create_file(".protowasmrc", "1.2.3");

            assert!(fixture.path().join(".protowasmrc").exists());

            assert_eq!(
                tool.detect_version_from(fixture.path()).await.unwrap(),
                Some("1.2.3".into())
            );
        }

        #[tokio::test]
        async fn matches_with_parser() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            fixture.create_file(".proto-wasm-version", "version=1.2.3");

            assert_eq!(
                tool.detect_version_from(fixture.path()).await.unwrap(),
                Some("1.2.3".into())
            );
        }

        #[tokio::test]
        async fn skips_if_parse_fails() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            fixture.create_file(".proto-wasm-version", "1.2.3");

            assert_eq!(
                tool.detect_version_from(fixture.path()).await.unwrap(),
                None
            );
        }
    }

    mod downloader {
        use super::*;

        #[tokio::test]
        async fn returns_download_url() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            assert_eq!(
                tool.get_download_url().unwrap(),
                format!("https://nodejs.org/dist/v20.0.0/{}", get_file())
            );
        }

        #[tokio::test]
        async fn returns_download_path() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            assert_eq!(
                tool.get_download_path().unwrap(),
                fixture.path().join("temp/wasm/20.0.0").join(get_file())
            );
        }

        #[tokio::test]
        async fn downloads_to_temp() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            let to_file = tool.get_download_path().unwrap();

            assert!(!to_file.exists());

            tool.download(&to_file, None).await.unwrap();

            assert!(to_file.exists());
        }
    }

    mod installer {
        use super::*;

        #[tokio::test]
        async fn sets_dir_to_tools() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            assert_eq!(
                tool.get_install_dir().unwrap(),
                Proto::from(fixture.path())
                    .tools_dir
                    .join("wasm")
                    .join("20.0.0")
            );
        }

        #[tokio::test]
        #[should_panic(expected = "InstallMissingDownload(\"WASM Test\")")]
        async fn errors_for_missing_download() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            let dir = tool.get_install_dir().unwrap();

            tool.install(&dir, &tool.get_download_path().unwrap())
                .await
                .unwrap();
        }

        #[tokio::test]
        async fn doesnt_install_if_dir_exists() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            let dir = tool.get_install_dir().unwrap();

            fs::create_dir_all(&dir).unwrap();

            assert!(!tool
                .install(&dir, &tool.get_download_path().unwrap())
                .await
                .unwrap());
        }

        #[tokio::test]
        async fn installs_and_unpacks() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin(fixture.path());

            let dir = tool.get_install_dir().unwrap();

            assert!(!dir.exists());

            tool.setup("20.0.0").await.unwrap();

            assert!(dir.exists());
        }
    }

    mod executor {
        use super::*;

        mod globals {
            use super::*;

            #[tokio::test]
            async fn returns_nothing_if_no_matching_vars() {
                let fixture = create_empty_sandbox();
                let tool = create_plugin(fixture.path());

                assert_eq!(tool.get_globals_bin_dir().unwrap(), None);
            }

            #[tokio::test]
            async fn expands_home_env_var() {
                let fixture = create_empty_sandbox();
                let tool = create_plugin(fixture.path());

                // Dir must exist!
                fixture.create_file(".wasm/bin/test", "");

                fixture.debug_files();

                env::set_var("HOME", fixture.path().to_string_lossy().to_string());
                env::set_var("PROTO_ROOT", fixture.path().to_string_lossy().to_string());

                assert_eq!(
                    tool.get_globals_bin_dir().unwrap().unwrap(),
                    fixture.path().join(".wasm/bin")
                );

                env::remove_var("PROTO_ROOT");
                env::remove_var("HOME");
            }

            #[tokio::test]
            async fn expands_custom_env_var() {
                let fixture = create_empty_sandbox();
                let tool = create_plugin(fixture.path());

                env::set_var("WASM_ROOT", fixture.path().to_string_lossy().to_string());

                // Dir must exist!
                fixture.create_file("bin/test", "");

                assert_eq!(
                    tool.get_globals_bin_dir().unwrap().unwrap(),
                    fixture.path().join("bin")
                );

                env::remove_var("WASM_ROOT");
            }
        }
    }

    mod resolver {
        use super::*;

        #[tokio::test]
        async fn loads_version_manifest() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());
            let manifest = tool.load_version_manifest().await.unwrap();

            assert!(!manifest.versions.is_empty());
            assert!(manifest.aliases.get("latest").is_some())
        }

        #[tokio::test]
        async fn resolve_latest() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin_without_version(fixture.path());

            assert_ne!(tool.resolve_version("latest").await.unwrap(), "latest");
        }

        #[tokio::test]
        async fn resolve_node_alias() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin_without_version(fixture.path());

            assert_ne!(tool.resolve_version("node").await.unwrap(), "node");
        }

        #[tokio::test]
        async fn resolve_version() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin_without_version(fixture.path());

            assert_eq!(tool.resolve_version("18.0.0").await.unwrap(), "18.0.0");
        }

        #[tokio::test]
        async fn resolve_partial_version() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin_without_version(fixture.path());

            assert_eq!(tool.resolve_version("10.1").await.unwrap(), "10.1.0");
        }

        #[tokio::test]
        #[should_panic(expected = "VersionUnknownAlias(\"unknown\")")]
        async fn errors_invalid_alias() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin_without_version(fixture.path());

            tool.resolve_version("unknown").await.unwrap();
        }

        #[tokio::test]
        #[should_panic(expected = "VersionResolveFailed(\"99.99.99\")")]
        async fn errors_invalid_version() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin_without_version(fixture.path());

            tool.resolve_version("99.99.99").await.unwrap();
        }
    }

    mod shimmer {
        use super::*;

        #[cfg(not(windows))]
        #[tokio::test]
        async fn creates_alt_globals() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin(fixture.path());

            env::set_var("PROTO_ROOT", fixture.path().to_string_lossy().to_string());

            // tool.find_bin_path().await.unwrap();
            tool.create_shims(false).await.unwrap();

            env::remove_var("PROTO_ROOT");

            let g1 = fixture.path().join("bin/global1");

            assert!(fixture.path().join("bin/wasm").exists());
            assert!(g1.exists());

            let g1 = fs::read_to_string(g1).unwrap();

            assert!(g1.contains(r#"exec proto run wasm --bin "bin/global1" --  "$@""#));
        }

        #[cfg(windows)]
        #[tokio::test]
        async fn creates_alt_globals() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin(fixture.path());

            env::set_var("PROTO_ROOT", fixture.path().to_string_lossy().to_string());

            // tool.find_bin_path().await.unwrap();
            tool.create_shims(false).await.unwrap();

            env::remove_var("PROTO_ROOT");

            let g1 = fixture.path().join("bin/global1.cmd");

            assert!(fixture.path().join("bin/wasm.cmd").exists());
            assert!(g1.exists());

            let g1 = fs::read_to_string(g1).unwrap();

            assert!(g1.contains(r#"proto.exe run wasm --bin "bin/global1" --  %*"#));
        }

        #[cfg(not(windows))]
        #[tokio::test]
        async fn creates_locals() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin(fixture.path());

            // tool.find_bin_path().await.unwrap();
            tool.create_shims(false).await.unwrap();

            let l1 = fixture.path().join("tools/wasm/20.0.0/shims/local1");
            let l2 = fixture.path().join("tools/wasm/20.0.0/shims/local2");

            assert!(l1.exists());
            assert!(l2.exists());

            let l1 = fs::read_to_string(l1).unwrap();
            let l2 = fs::read_to_string(l2).unwrap();

            assert!(l1.contains(r#"parent="${PROTO_NODE_BIN:-node}""#));
            assert!(l1.contains(&format!(
                r#"exec "$parent" "{}"  "$@""#,
                tool.get_install_dir()
                    .unwrap()
                    .join("bin/local1")
                    .to_string_lossy()
            )));

            assert!(l2.contains(&format!(
                r#"exec "{}"  "$@""#,
                tool.get_install_dir()
                    .unwrap()
                    .join("local2.js")
                    .to_string_lossy()
            )));
        }

        #[cfg(windows)]
        #[tokio::test]
        async fn creates_locals() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin(fixture.path());

            // tool.find_bin_path().await.unwrap();
            tool.create_shims(false).await.unwrap();

            let l1 = fixture.path().join("tools/wasm/20.0.0/shims/local1.ps1");
            let l2 = fixture.path().join("tools/wasm/20.0.0/shims/local2.ps1");

            assert!(l1.exists());
            assert!(l2.exists());

            let l1 = fs::read_to_string(l1).unwrap();
            let l2 = fs::read_to_string(l2).unwrap();

            assert!(l1.contains(r#"$parent = "node""#));
            assert!(l1.contains(&format!(
                r#"& "$parent" "{}"  $args"#,
                tool.get_install_dir()
                    .unwrap()
                    .join("bin/local1")
                    .to_string_lossy()
            )));

            assert!(l2.contains(&format!(
                r#"& "{}"  $args"#,
                tool.get_install_dir()
                    .unwrap()
                    .join("local2.js")
                    .to_string_lossy()
            )));
        }
    }

    mod verifier {
        use super::*;

        #[tokio::test]
        async fn returns_checksum_url() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            assert_eq!(
                tool.get_checksum_url().unwrap().unwrap(),
                "https://nodejs.org/dist/v20.0.0/SHASUMS256.txt"
            );
        }

        #[tokio::test]
        async fn returns_checksum_path() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            assert_eq!(
                tool.get_checksum_path().unwrap(),
                fixture.path().join("temp/wasm/20.0.0/CHECKSUM.txt")
            );
        }

        #[tokio::test]
        async fn downloads_to_temp() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());

            let to_file = tool.get_checksum_path().unwrap();

            assert!(!to_file.exists());

            tool.download_checksum(&to_file, None).await.unwrap();

            assert!(to_file.exists());
        }

        #[tokio::test]
        async fn passes_for_checksum_match() {
            let fixture = create_empty_sandbox();
            let tool = create_plugin(fixture.path());
            let dl_path = tool.get_download_path().unwrap();
            let cs_path = tool.get_checksum_path().unwrap();

            tool.download(&dl_path, None).await.unwrap();
            tool.download_checksum(&cs_path, None).await.unwrap();
            tool.verify_checksum(&cs_path, &dl_path).await.unwrap();
        }

        #[tokio::test]
        #[should_panic(expected = "VerifyInvalidChecksum")]
        async fn errors_for_checksum_mismatch() {
            let fixture = create_empty_sandbox();
            let mut tool = create_plugin(fixture.path());

            // Tests pass for version 20 and fails for others
            tool.version = Some("19.0.0".into());

            let dl_path = tool.get_download_path().unwrap();
            let cs_path = tool.get_checksum_path().unwrap();

            tool.download(&dl_path, None).await.unwrap();
            tool.download_checksum(&cs_path, None).await.unwrap();

            // Empty the checksum file
            std::fs::write(&cs_path, "").unwrap();

            tool.verify_checksum(&cs_path, &dl_path).await.unwrap();
        }
    }
}
