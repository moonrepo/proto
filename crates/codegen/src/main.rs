use proto_core::registry::PluginRegistryDocument;
use proto_pdk_api::*;
use schematic::schema::typescript::TypeScriptRenderer;
use schematic::schema::{JsonSchemaRenderer, SchemaGenerator};
use std::fs;
use std::path::PathBuf;

// cargo run -p proto_codegen
fn generate_types() {
    let mut generator = SchemaGenerator::default();

    // system_env
    generator.add::<HostArch>();
    generator.add::<HostOS>();
    generator.add::<HostLibc>();
    generator.add::<HostPackageManager>();

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

    // proto
    generator.add::<PluginContext>();
    generator.add::<PluginUnresolvedContext>();
    generator.add::<PluginType>();
    generator.add::<ToolInventoryMetadata>();
    generator.add::<RegisterToolInput>();
    generator.add::<RegisterToolOutput>();
    generator.add::<RegisterBackendInput>();
    generator.add::<RegisterBackendOutput>();
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

    generator.add::<BuildInstructionsInput>();
    generator.add::<SourceLocation>();
    generator.add::<BuildInstruction>();
    generator.add::<BuildRequirement>();
    generator.add::<BuildInstructionsOutput>();
    generator.add::<BuildInstructionsInput>();

    generator
        .generate(
            PathBuf::from("package/src/api-types.ts"),
            TypeScriptRenderer::default(),
        )
        .unwrap();
}

fn generate_registry_schema() {
    let mut generator = SchemaGenerator::default();
    generator.add::<PluginRegistryDocument>();
    generator
        .generate(
            PathBuf::from("registry/schema.json"),
            JsonSchemaRenderer::default(),
        )
        .unwrap();
}

fn load_save_json(path: &str) {
    let mut json: PluginRegistryDocument =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();

    json.plugins.sort_by(|a, d| a.id.cmp(&d.id));

    fs::write(path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
}

fn validate_registries() {
    load_save_json("registry/data/built-in.json");
    load_save_json("registry/data/third-party.json");
}

fn main() {
    generate_types();
    generate_registry_schema();
    validate_registries();
}
