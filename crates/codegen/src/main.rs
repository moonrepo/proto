use proto_pdk_api::*;
use schematic::schema::typescript::TypeScriptRenderer;
use schematic::schema::SchemaGenerator;
use std::path::PathBuf;

// cargo run -p proto_codegen
fn main() {
    let mut generator = SchemaGenerator::default();

    // proto
    generator.add::<ToolContext>();
    generator.add::<PluginType>();
    generator.add::<ToolMetadataInput>();
    generator.add::<ToolInventoryMetadata>();
    generator.add::<ToolMetadataOutput>();
    generator.add::<DetectVersionOutput>();
    generator.add::<ParseVersionFileInput>();
    generator.add::<ParseVersionFileOutput>();
    generator.add::<NativeInstallInput>();
    generator.add::<NativeInstallOutput>();
    generator.add::<NativeUninstallInput>();
    generator.add::<NativeUninstallOutput>();
    generator.add::<DownloadPrebuiltInput>();
    generator.add::<DownloadPrebuiltOutput>();
    generator.add::<UnpackArchiveInput>();
    generator.add::<VerifyChecksumInput>();
    generator.add::<VerifyChecksumOutput>();
    generator.add::<LocateExecutablesInput>();
    generator.add::<ExecutableConfig>();
    generator.add::<LocateExecutablesOutput>();
    generator.add::<LoadVersionsInput>();
    generator.add::<LoadVersionsOutput>();
    generator.add::<ResolveVersionInput>();
    generator.add::<ResolveVersionOutput>();
    generator.add::<SyncManifestInput>();
    generator.add::<SyncManifestOutput>();
    generator.add::<SyncShellProfileInput>();
    generator.add::<SyncShellProfileOutput>();
    generator.add::<InstallHook>();
    generator.add::<RunHook>();
    generator.add::<RunHookResult>();

    // system_env
    generator.add::<HostArch>();
    generator.add::<HostOS>();
    generator.add::<HostLibc>();

    // version_spec
    generator.add::<VersionSpec>();
    generator.add::<UnresolvedVersionSpec>();

    // warpgate
    generator.add::<HostLogTarget>();
    generator.add::<HostLogInput>();
    generator.add::<ExecCommandInput>();
    generator.add::<ExecCommandOutput>();
    generator.add::<HostEnvironment>();
    generator.add::<TestEnvironment>();
    generator.add::<PluginLocator>();
    generator.add::<VirtualPath>();

    generator
        .generate(
            PathBuf::from("package/src/api-types.ts"),
            TypeScriptRenderer::default(),
        )
        .unwrap();
}
