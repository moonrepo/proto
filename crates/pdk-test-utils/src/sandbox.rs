use crate::wrapper::WasmTestWrapper;
use proto_core::{ProtoEnvironment, Tool, ToolContext, inject_proto_manifest_config};
use starbase_sandbox::{Sandbox, create_empty_sandbox, create_sandbox};
use std::fmt;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use warpgate::test_utils::*;
use warpgate::{Wasm, inject_default_manifest_config};

pub struct ProtoWasmSandbox {
    pub sandbox: Sandbox,
    pub home_dir: PathBuf,
    pub proto_dir: PathBuf,
    pub root: PathBuf,
    pub wasm_file: PathBuf,
}

impl ProtoWasmSandbox {
    pub fn new(sandbox: Sandbox) -> Self {
        let root = sandbox.path().to_path_buf();
        let home_dir = root.join(".home");
        let proto_dir = root.join(".proto");
        let wasm_file = find_wasm_file();

        // Folders must exist for WASM to compile correctly!
        fs::create_dir_all(&home_dir).unwrap();
        fs::create_dir_all(&proto_dir).unwrap();

        Self {
            home_dir,
            proto_dir,
            root,
            sandbox,
            wasm_file,
        }
    }

    pub fn create_config(&self) -> ConfigBuilder {
        ConfigBuilder::new(&self.root, &self.home_dir)
    }

    pub async fn create_plugin(&self, context: &str) -> WasmTestWrapper {
        self.create_plugin_with_config(context, |_| {}).await
    }

    pub async fn create_plugin_with_config(
        &self,
        context: &str,
        mut op: impl FnMut(&mut ConfigBuilder),
    ) -> WasmTestWrapper {
        let context = ToolContext::parse(context).unwrap();
        let mut proto = ProtoEnvironment::new_testing(&self.root).unwrap();
        proto.working_dir = self.root.clone();

        // Create manifest
        let mut manifest =
            Tool::create_plugin_manifest(&proto, Wasm::file(&self.wasm_file)).unwrap();

        inject_default_manifest_config(&context.id, &proto.home_dir, &mut manifest).unwrap();
        inject_proto_manifest_config(&context.id, &proto, &mut manifest).unwrap();

        // Create config
        let mut config = self.create_config();
        op(&mut config);

        manifest.config.extend(config.build());

        WasmTestWrapper {
            tool: Tool::load_from_manifest(context, proto, manifest)
                .await
                .unwrap(),
        }
    }

    pub async fn create_schema_plugin(
        &self,
        context: &str,
        schema_path: PathBuf,
    ) -> WasmTestWrapper {
        self.create_schema_plugin_with_config(context, schema_path, |_| {})
            .await
    }

    #[allow(unused_variables)]
    pub async fn create_schema_plugin_with_config(
        &self,
        context: &str,
        schema_path: PathBuf,
        mut op: impl FnMut(&mut ConfigBuilder),
    ) -> WasmTestWrapper {
        self.create_plugin_with_config(context, move |config| {
            op(config);

            #[cfg(feature = "schema")]
            {
                use crate::config_builder::ProtoConfigBuilder;

                config.schema_config(proto_core::load_schema_config(&schema_path).unwrap());
            }
        })
        .await
    }

    pub fn enable_logging(&self) {
        enable_wasm_logging(&self.wasm_file);
    }
}

impl Deref for ProtoWasmSandbox {
    type Target = Sandbox;

    fn deref(&self) -> &Self::Target {
        &self.sandbox
    }
}

impl fmt::Debug for ProtoWasmSandbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProtoSandbox")
            .field("home_dir", &self.home_dir)
            .field("proto_dir", &self.proto_dir)
            .field("root", &self.root)
            .field("wasm_file", &self.wasm_file)
            .finish()
    }
}

pub fn create_proto_sandbox(fixture: &str) -> ProtoWasmSandbox {
    ProtoWasmSandbox::new(create_sandbox(fixture))
}

pub fn create_empty_proto_sandbox() -> ProtoWasmSandbox {
    ProtoWasmSandbox::new(create_empty_sandbox())
}
