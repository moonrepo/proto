use proto_core::Installable;
use proto_pdk_api::*;
use proto_wasm_plugin::WasmPlugin;
use std::path::{Path, PathBuf};

pub struct WasmTestWrapper {
    pub tool: WasmPlugin,
}

impl WasmTestWrapper {
    pub fn to_virtual_path(&self, path: &Path) -> PathBuf {
        self.tool.to_wasi_virtual_path(path)
    }

    pub fn create_shims(&self, input: CreateShimsInput) -> CreateShimsOutput {
        self.tool.call_func_with("create_shims", input).unwrap()
    }

    pub fn detect_version(&self) -> DetectVersionOutput {
        self.tool.call_func("detect_version").unwrap()
    }

    pub fn download_prebuilt(&self, input: DownloadPrebuiltInput) -> DownloadPrebuiltOutput {
        self.tool
            .call_func_with("download_prebuilt", input)
            .unwrap()
    }

    pub fn load_versions(&self, input: LoadVersionsInput) -> LoadVersionsOutput {
        self.tool.call_func_with("load_versions", input).unwrap()
    }

    pub fn locate_bins(&self, mut input: LocateBinsInput) -> LocateBinsOutput {
        if input.tool_dir.components().count() == 0 {
            input.tool_dir = self.tool.get_install_dir().unwrap();
        }

        input.tool_dir = self.to_virtual_path(&input.tool_dir);

        self.tool.call_func_with("locate_bins", input).unwrap()
    }

    pub fn parse_version_file(&self, input: ParseVersionFileInput) -> ParseVersionFileOutput {
        self.tool
            .call_func_with("parse_version_file", input)
            .unwrap()
    }

    pub fn register_tool(&self, input: ToolMetadataInput) -> ToolMetadataOutput {
        self.tool.call_func_with("register_tool", input).unwrap()
    }

    pub fn resolve_version(&self, input: ResolveVersionInput) -> ResolveVersionOutput {
        self.tool.call_func_with("resolve_version", input).unwrap()
    }

    pub fn unpack_archive(&self, mut input: UnpackArchiveInput) {
        input.input_file = self.to_virtual_path(&input.input_file);
        input.output_dir = self.to_virtual_path(&input.output_dir);

        let _: EmptyInput = self.tool.call_func_with("unpack_archive", input).unwrap();
    }

    pub fn verify_checksum(&self, mut input: VerifyChecksumInput) -> VerifyChecksumOutput {
        input.checksum_file = self.to_virtual_path(&input.checksum_file);
        input.download_file = self.to_virtual_path(&input.download_file);

        self.tool.call_func_with("verify_checksum", input).unwrap()
    }
}
