mod utils;

use futures::Future;
use proto::{tools::create_plugin_from_locator, ProtoError};
use proto_core::{PluginLocation, PluginLocator, Proto, Tool};
use std::env;
use std::path::{Path, PathBuf};
use utils::*;

async fn run_tests<F, Fut>(factory: F)
where
    F: FnOnce(&Path) -> Fut,
    Fut: Future<Output = Result<Box<dyn Tool<'static>>, ProtoError>>,
{
    let fixture = create_empty_sandbox();
    let proto = Proto::from(fixture.path());

    let mut tool = factory(fixture.path()).await.unwrap();

    env::set_var("PROTO_ROOT", fixture.path().to_string_lossy().to_string());

    tool.setup("1.0.0").await.unwrap();

    env::remove_var("PROTO_ROOT");

    assert!(tool.get_install_dir().unwrap().exists());

    let base_dir = proto.tools_dir.join("moon/1.0.0");

    if cfg!(windows) {
        assert_eq!(tool.get_bin_path().unwrap(), &base_dir.join("moon.exe"));
        assert!(proto.bin_dir.join("moon.cmd").exists());
    } else {
        assert_eq!(tool.get_bin_path().unwrap(), &base_dir.join("moon"));
        assert!(proto.bin_dir.join("moon").exists());
    }
}

#[tokio::test]
async fn downloads_and_installs_plugin_from_file() {
    run_tests(|root| {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        create_plugin_from_locator(
            "moon",
            Proto::from(root),
            PluginLocator::Source(PluginLocation::File(
                "./tests/fixtures/moon-schema.toml".into(),
            )),
            root_dir,
        )
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "PluginFileMissing")]
async fn errors_for_missing_file() {
    run_tests(|root| {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        create_plugin_from_locator(
            "moon",
            Proto::from(root),
            PluginLocator::Source(PluginLocation::File("./some/fake/path.toml".into())),
            root_dir,
        )
    })
    .await;
}

#[tokio::test]
async fn downloads_and_installs_plugin_from_url() {
    run_tests(|root| {
        create_plugin_from_locator(
            "moon",
            Proto::from(root),
            PluginLocator::Source(PluginLocation::Url(
                "https://raw.githubusercontent.com/moonrepo/moon/master/proto-plugin.toml".into(),
            )),
            PathBuf::new(),
        )
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "DownloadNotFound")]
async fn errors_for_broken_url() {
    run_tests(|root| {
        create_plugin_from_locator(
            "moon",
            Proto::from(root),
            PluginLocator::Source(PluginLocation::Url(
                "https://raw.githubusercontent.com/moonrepo/moon/some/fake/path.toml".into(),
            )),
            PathBuf::new(),
        )
    })
    .await;
}
