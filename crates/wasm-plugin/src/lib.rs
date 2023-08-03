mod host_funcs;

pub use host_funcs::*;

pub struct WasmPlugin {
    pub id: String,
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub shim_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,

    pub container: PluginContainer<'static>,
    manifest: OnceCell<Manifest>,
    proto: Proto,
}

impl WasmPlugin {
    pub fn new<P: AsRef<Proto>, L: AsRef<Path>>(
        proto: P,
        id: String,
        wasm_file: L,
    ) -> Result<Self, ProtoError> {
        let proto = proto.as_ref();
        let working_dir = env::current_dir().unwrap();
        let plugin_paths = FxHashMap::from_iter([
            (PathBuf::from("/workspace"), working_dir.clone()),
            (PathBuf::from("/home"), proto.home.clone()),
            (PathBuf::from("/proto"), proto.root.clone()),
        ]);

        let mut manifest = PluginManifest::new([Wasm::file(wasm_file)]);
        manifest = manifest.with_allowed_host("*");

        #[cfg(debug_assertions)]
        {
            manifest = manifest.with_timeout(Duration::from_secs(90));
        }

        for (virtual_path, real_path) in &plugin_paths {
            manifest = manifest.with_allowed_path(real_path, virtual_path);
        }

        let host_data = HostData { working_dir };

        let wasm_plugin = WasmPlugin {
            base_dir: proto.tools_dir.join(&id),
            bin_path: None,
            container: PluginContainer::new(&id, manifest, host_funcs::create_functions(host_data))
                .map_err(|e| ProtoError::Message(e.to_string()))?,
            manifest: OnceCell::new(),
            shim_path: None,
            temp_dir: proto.temp_dir.join(&id),
            version: None,
            id,
            proto: proto.to_owned(),
        };

        // Load metadata on load and make available
        wasm_plugin.get_metadata()?;

        Ok(wasm_plugin)
    }
}
