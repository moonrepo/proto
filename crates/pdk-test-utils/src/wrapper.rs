use proto_core::Tool;
use proto_pdk_api::*;

#[derive(Debug)]
pub struct WasmTestWrapper {
    pub tool: Tool,
}

impl WasmTestWrapper {
    pub async fn detect_version_files(&self, mut input: DetectVersionInput) -> DetectVersionOutput {
        input.context = self.prepare_unresolved_context(input.context);

        self.tool
            .plugin
            .call_func_with(PluginFunction::DetectVersionFiles, input)
            .await
            .unwrap()
    }

    pub async fn download_prebuilt(
        &self,
        mut input: DownloadPrebuiltInput,
    ) -> DownloadPrebuiltOutput {
        input.context = self.prepare_context(input.context);

        self.tool
            .plugin
            .call_func_with(PluginFunction::DownloadPrebuilt, input)
            .await
            .unwrap()
    }

    pub async fn load_versions(&self, mut input: LoadVersionsInput) -> LoadVersionsOutput {
        input.context = self.prepare_unresolved_context(input.context);

        self.tool
            .plugin
            .call_func_with(PluginFunction::LoadVersions, input)
            .await
            .unwrap()
    }

    pub async fn locate_executables(
        &self,
        mut input: LocateExecutablesInput,
    ) -> LocateExecutablesOutput {
        input.context = self.prepare_context(input.context);

        self.tool
            .plugin
            .call_func_with(PluginFunction::LocateExecutables, input)
            .await
            .unwrap()
    }

    pub async fn native_install(&self, mut input: NativeInstallInput) -> NativeInstallOutput {
        input.context = self.prepare_context(input.context);

        self.tool
            .plugin
            .call_func_with(PluginFunction::NativeInstall, input)
            .await
            .unwrap()
    }

    pub async fn native_uninstall(&self, mut input: NativeUninstallInput) -> NativeUninstallOutput {
        input.context = self.prepare_context(input.context);

        self.tool
            .plugin
            .call_func_with(PluginFunction::NativeUninstall, input)
            .await
            .unwrap()
    }

    pub async fn parse_version_file(
        &self,
        mut input: ParseVersionFileInput,
    ) -> ParseVersionFileOutput {
        input.context = self.prepare_unresolved_context(input.context);
        input.path = self.tool.to_virtual_path(&input.path);

        self.tool
            .plugin
            .call_func_with(PluginFunction::ParseVersionFile, input)
            .await
            .unwrap()
    }

    pub async fn pre_install(&self, mut input: InstallHook) {
        input.context = self.prepare_context(input.context);

        self.tool
            .plugin
            .call_func_without_output(HookFunction::PreInstall, input)
            .await
            .unwrap();
    }

    pub async fn pre_run(&self, mut input: RunHook) -> RunHookResult {
        input.context = self.prepare_context(input.context);

        self.tool
            .plugin
            .call_func_with(HookFunction::PreRun, input)
            .await
            .unwrap()
    }

    pub async fn post_install(&self, mut input: InstallHook) {
        input.context = self.prepare_context(input.context);

        self.tool
            .plugin
            .call_func_without_output(HookFunction::PostInstall, input)
            .await
            .unwrap();
    }

    pub async fn register_backend(&self, mut input: RegisterBackendInput) -> RegisterBackendOutput {
        input.context = self.prepare_unresolved_context(input.context);

        self.tool
            .plugin
            .call_func_with(PluginFunction::RegisterBackend, input)
            .await
            .unwrap()
    }

    pub async fn register_tool(&self, input: RegisterToolInput) -> RegisterToolOutput {
        self.tool
            .plugin
            .call_func_with(PluginFunction::RegisterTool, input)
            .await
            .unwrap()
    }

    pub async fn resolve_version(&self, mut input: ResolveVersionInput) -> ResolveVersionOutput {
        input.context = self.prepare_unresolved_context(input.context);

        self.tool
            .plugin
            .call_func_with(PluginFunction::ResolveVersion, input)
            .await
            .unwrap()
    }

    pub async fn sync_manifest(&self, mut input: SyncManifestInput) -> SyncManifestOutput {
        input.context = self.prepare_context(input.context);

        self.tool
            .plugin
            .call_func_with(PluginFunction::SyncManifest, input)
            .await
            .unwrap()
    }

    pub async fn sync_shell_profile(
        &self,
        mut input: SyncShellProfileInput,
    ) -> SyncShellProfileOutput {
        input.context = self.prepare_context(input.context);

        self.tool
            .plugin
            .call_func_with(PluginFunction::SyncShellProfile, input)
            .await
            .unwrap()
    }

    pub async fn unpack_archive(&self, mut input: UnpackArchiveInput) {
        input.input_file = self.tool.to_virtual_path(&input.input_file);
        input.output_dir = self.tool.to_virtual_path(&input.output_dir);

        let _: EmptyInput = self
            .tool
            .plugin
            .call_func_with(PluginFunction::UnpackArchive, input)
            .await
            .unwrap();
    }

    pub async fn verify_checksum(&self, mut input: VerifyChecksumInput) -> VerifyChecksumOutput {
        input.checksum_file = self.tool.to_virtual_path(&input.checksum_file);
        input.download_file = self.tool.to_virtual_path(&input.download_file);

        self.tool
            .plugin
            .call_func_with(PluginFunction::VerifyChecksum, input)
            .await
            .unwrap()
    }

    fn prepare_context(&self, context: PluginContext) -> PluginContext {
        let tool_dir = if context.tool_dir.any_path().components().count() == 0 {
            self.tool.get_product_dir()
        } else {
            context.tool_dir.any_path().to_path_buf()
        };

        let temp_dir = if context.temp_dir.any_path().components().count() == 0 {
            self.tool.get_temp_dir()
        } else {
            context.temp_dir.any_path().to_path_buf()
        };

        PluginContext {
            temp_dir: self.tool.to_virtual_path(&temp_dir),
            tool_dir: self.tool.to_virtual_path(&tool_dir),
            ..context
        }
    }

    fn prepare_unresolved_context(
        &self,
        context: PluginUnresolvedContext,
    ) -> PluginUnresolvedContext {
        let temp_dir = if context.temp_dir.any_path().components().count() == 0 {
            self.tool.get_temp_dir()
        } else {
            context.temp_dir.any_path().to_path_buf()
        };

        PluginUnresolvedContext {
            temp_dir: self.tool.to_virtual_path(&temp_dir),
            ..context
        }
    }
}
