use crate::wrapper::WasmTestWrapper;
use proto_core::{inject_proto_manifest_config, ProtoEnvironment, Tool};
use starbase_sandbox::Sandbox;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Mutex;
use std::{fmt, fs};
use warpgate::test_utils::*;
use warpgate::{inject_default_manifest_config, Id, Wasm};

pub struct ProtoSandbox {
    pub sandbox: Sandbox,
    pub home_dir: PathBuf,
    pub proto_dir: PathBuf,
    pub root: PathBuf,
    pub wasm_file: PathBuf,
    pub wasm_logs: Rc<Mutex<Vec<String>>>,
}

impl ProtoSandbox {
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
            wasm_logs: Rc::new(Mutex::new(vec![])),
        }
    }

    pub fn debug_logs(&self) {
        println!(
            "WASM LOGS:\n{}\n",
            self.wasm_logs.lock().unwrap().join("\n")
        );
    }

    pub fn create_config(&self) -> ConfigBuilder {
        ConfigBuilder::new(&self.root, &self.home_dir)
    }

    pub fn create_plugin(&self, id: &str) -> WasmTestWrapper {
        self.create_plugin_with_config(id, |_| {})
    }

    pub fn create_plugin_with_config(
        &self,
        id: &str,
        mut op: impl FnMut(&mut ConfigBuilder),
    ) -> WasmTestWrapper {
        let id = Id::new(id).unwrap();
        let proto = ProtoEnvironment::new_testing(&self.root);

        // Create manifest
        let mut manifest =
            Tool::create_plugin_manifest(&proto, Wasm::file(&self.wasm_file)).unwrap();

        inject_default_manifest_config(&id, &proto.home, &mut manifest).unwrap();
        inject_proto_manifest_config(&id, &proto, &mut manifest).unwrap();

        // Create config
        let mut config = self.create_config();
        op(&mut config);

        manifest.config.extend(config.build());

        // Track logs
        let logs = Rc::clone(&self.wasm_logs);

        let _ = extism::set_log_callback(
            move |line| {
                if line.contains("extism::")
                    || line.contains("warpgate::")
                    || line.contains("proto")
                {
                    logs.lock().unwrap().push(line.to_owned());
                }
            },
            "debug",
        );

        WasmTestWrapper {
            tool: Tool::load_from_manifest(id, proto, manifest).unwrap(),
        }
    }

    pub fn create_schema_plugin(&self, id: &str, schema: PathBuf) -> WasmTestWrapper {
        self.create_schema_plugin_with_config(id, schema, |_| {})
    }

    #[allow(unused_variables)]
    pub fn create_schema_plugin_with_config(
        &self,
        id: &str,
        schema: PathBuf,
        mut op: impl FnMut(&mut ConfigBuilder),
    ) -> WasmTestWrapper {
        self.create_plugin_with_config(id, |config| {
            op(config);

            #[cfg(feature = "schema")]
            {
                use crate::config_builder::ProtoConfigBuilder;

                let schema = fs::read_to_string(schema).unwrap();
                let schema: serde_json::Value = toml::from_str(&schema).unwrap();

                config.toml_schema(schema);
            }
        })
    }
}

impl Deref for ProtoSandbox {
    type Target = Sandbox;

    fn deref(&self) -> &Self::Target {
        &self.sandbox
    }
}

impl fmt::Debug for ProtoSandbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProtoSandbox")
            .field("home_dir", &self.home_dir)
            .field("proto_dir", &self.proto_dir)
            .field("root", &self.root)
            .field("wasm_file", &self.wasm_file)
            .field("wasm_logs", &self.wasm_logs)
            .finish()
    }
}

pub fn create_sandbox(fixture: &str) -> ProtoSandbox {
    ProtoSandbox::new(starbase_sandbox::create_sandbox(fixture))
}

pub fn create_empty_sandbox() -> ProtoSandbox {
    ProtoSandbox::new(starbase_sandbox::create_empty_sandbox())
}
