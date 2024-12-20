use crate::wrapper::WasmTestWrapper;
use proto_core::{inject_proto_manifest_config, ProtoEnvironment, Tool};
use starbase_sandbox::{create_empty_sandbox, create_sandbox, Sandbox};
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::{fmt, fs};
use warpgate::test_utils::*;
use warpgate::{inject_default_manifest_config, Id, Wasm};

pub struct ProtoWasmSandbox {
    pub sandbox: Sandbox,
    pub home_dir: PathBuf,
    pub proto_dir: PathBuf,
    pub root: PathBuf,
    pub wasm_file: PathBuf,
    pub wasm_logs: Rc<Mutex<Vec<String>>>,

    dropped: Rc<AtomicBool>,
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
            wasm_logs: Rc::new(Mutex::new(vec![])),
            dropped: Rc::new(AtomicBool::new(false)),
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

    pub async fn create_plugin(&self, id: &str) -> WasmTestWrapper {
        self.create_plugin_with_config(id, |_| {}).await
    }

    pub async fn create_plugin_with_config(
        &self,
        id: &str,
        mut op: impl FnMut(&mut ConfigBuilder),
    ) -> WasmTestWrapper {
        let id = Id::new(id).unwrap();
        let mut proto = ProtoEnvironment::new_testing(&self.root).unwrap();
        proto.working_dir = self.root.clone();

        // Create manifest
        let mut manifest =
            Tool::create_plugin_manifest(&proto, Wasm::file(&self.wasm_file)).unwrap();

        inject_default_manifest_config(&id, &proto.home_dir, &mut manifest).unwrap();
        inject_proto_manifest_config(&id, &proto, &mut manifest).unwrap();

        // Create config
        let mut config = self.create_config();
        op(&mut config);

        manifest.config.extend(config.build());

        // Track logs
        // let logs = Rc::clone(&self.wasm_logs);
        // let dropped = Rc::clone(&self.dropped);

        // let _ = extism::set_log_callback(
        //     move |line| {
        //         if dropped.load(Ordering::Relaxed) == false
        //             && !line.is_empty()
        //             && (line.contains("extism::")
        //                 || line.contains("warpgate::")
        //                 || line.contains("proto"))
        //         {
        //             // Test may have finished but this closure is still executing,
        //             // so don't unwrap() and avoid any failures
        //             if let Ok(mut lock) = logs.try_lock() {
        //                 lock.push(line.to_owned());
        //             }
        //         }
        //     },
        //     "debug",
        // );

        WasmTestWrapper {
            tool: Tool::load_from_manifest(id, proto, manifest).await.unwrap(),
        }
    }

    pub async fn create_schema_plugin(&self, id: &str, schema_path: PathBuf) -> WasmTestWrapper {
        self.create_schema_plugin_with_config(id, schema_path, |_| {})
            .await
    }

    #[allow(unused_variables)]
    pub async fn create_schema_plugin_with_config(
        &self,
        id: &str,
        schema_path: PathBuf,
        mut op: impl FnMut(&mut ConfigBuilder),
    ) -> WasmTestWrapper {
        self.create_plugin_with_config(id, move |config| {
            op(config);

            #[cfg(feature = "schema")]
            {
                use crate::config_builder::ProtoConfigBuilder;

                config.schema_config(proto_core::load_schema_config(&schema_path).unwrap());
            }
        })
        .await
    }
}

impl Drop for ProtoWasmSandbox {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release)
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
            .field("wasm_logs", &self.wasm_logs)
            .finish()
    }
}

pub fn create_proto_sandbox(fixture: &str) -> ProtoWasmSandbox {
    ProtoWasmSandbox::new(create_sandbox(fixture))
}

pub fn create_empty_proto_sandbox() -> ProtoWasmSandbox {
    ProtoWasmSandbox::new(create_empty_sandbox())
}
