use dirs;
use extism_pdk::*;
use proto_core::Tool;
use proto_pdk::*;
use serde::Deserialize;
use std::env;
use std::path::Path;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct AsdfConfig {
    pub asdf_plugin: Option<String>,
    pub asdf_repository: Option<String>,
}

pub struct AsdfPlugin {
    pub tool: Tool,
}

impl AsdfPlugin {
    pub fn detect_version_files(&self) -> DetectVersionOutput {
        DetectVersionOutput {
            files: vec![".tool-versions".into()],
            ignore: vec![],
        }
    }

    pub fn parse_version_file(&self, input: ParseVersionFileInput) -> ParseVersionFileOutput {
        let mut version = None;
        if input.file == ".tool-versions" {
            for line in input.content.lines() {
                if let Some((tool, version_str)) = line.split_once(' ') {
                    if tool == self.tool.get_name() {
                        version = Some(UnresolvedVersionSpec::parse(version_str.trim()).unwrap());
                        break;
                    }
                }
            }
        }
        ParseVersionFileOutput { version }
    }

    pub fn download_prebuilt(&self, input: DownloadPrebuiltInput) -> DownloadPrebuiltOutput {
        let env = get_host_environment().unwrap();

        check_supported_os_and_arch(
            "ASDF Plugin",
            &env,
            permutations![
                HostOS::Linux => [HostArch::X64, HostArch::Arm64, HostArch::Arm, HostArch::Powerpc64, HostArch::S390x],
                HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
                HostOS::Windows => [HostArch::X64, HostArch::X86, HostArch::Arm64],
            ],
        ).unwrap();

        let version = input.context.version;
        let arch = env.arch;
        let os = env.os;

        let prefix = match os {
            HostOS::Linux => format!("asdf-plugin-v{version}-linux-{arch}"),
            HostOS::MacOS => format!("asdf-plugin-v{version}-darwin-{arch}"),
            HostOS::Windows => format!("asdf-plugin-v{version}-win-{arch}"),
            other => {
                return DownloadPrebuiltOutput {
                    download_url: format!("Unsupported platform: {}", other),
                    ..DownloadPrebuiltOutput::default()
                };
            }
        };

        let filename = if os == HostOS::Windows {
            format!("{prefix}.zip")
        } else {
            format!("{prefix}.tar.xz")
        };

        let config: AsdfConfig = self.tool.config();
        let asdf_plugin = config
            .asdf_plugin
            .unwrap_or_else(|| self.tool.get_name().to_string());
        let repository = config
            .asdf_repository
            .unwrap_or_else(|| format!("https://github.com/asdf-vm/asdf-{}.git", asdf_plugin));

        DownloadPrebuiltOutput {
            archive_prefix: Some(prefix),
            download_url: format!("{repository}/releases/download/v{version}/{filename}"),
            download_name: Some(filename),
            checksum_url: Some(format!(
                "{repository}/releases/download/v{version}/SHA256SUMS"
            )),
            checksum_public_key: Some("public-key-string".into()), // Need to adjust if applicable
            ..DownloadPrebuiltOutput::default()
        }
    }

    pub fn install_plugin(&self, repository: &str) {
        let asdf_dir = match env::var("ASDF_DATA_DIR") {
            Ok(val) => Path::new(&val).to_path_buf(),
            Err(_) => dirs::home_dir().unwrap().join(".asdf"),
        };

        let plugin_dir = asdf_dir.join("plugins").join(self.tool.get_name());

        if !plugin_dir.exists() {
            std::fs::create_dir_all(&plugin_dir).unwrap();
        }

        std::process::Command::new("git")
            .arg("clone")
            .arg(repository)
            .arg(&plugin_dir)
            .output()
            .expect("Failed to clone asdf plugin");
    }

    pub fn pre_install(&self, mut input: InstallHook) {
        let config: AsdfConfig = self.tool.config();
        let repository = config.asdf_repository.unwrap_or_else(|| {
            format!(
                "https://github.com/asdf-vm/asdf-{}.git",
                self.tool.get_name()
            )
        });
        self.install_plugin(&repository);
        input.context = self.prepare_context(input.context);
        self.tool
            .plugin
            .call_func_without_output("pre_install", input)
            .unwrap();
    }

    fn prepare_context(&self, context: ToolContext) -> ToolContext {
        let dir = if context.tool_dir.any_path().components().count() == 0 {
            self.tool.get_product_dir()
        } else {
            context.tool_dir.any_path().to_path_buf()
        };
        ToolContext {
            tool_dir: self.tool.to_virtual_path(&dir),
            ..context
        }
    }
}

// register_tool: Registers the plugin and provides metadata.
#[plugin_fn]
pub fn register_tool(Json(input): Json<ToolMetadataInput>) -> FnResult<Json<ToolMetadataOutput>> {
    Ok(Json(ToolMetadataOutput {
        name: "ASDF Plugin".into(),
        type_of: PluginType::Language,
        plugin_version: Some(env!("CARGO_PKG_VERSION").into()),
        ..ToolMetadataOutput::default()
    }))
}

// download_prebuilt: Handles downloading the pre-built tool, with URL construction based on OS and architecture.
#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let asdf_plugin = AsdfPlugin {
        tool: Tool::default(),
    };
    Ok(Json(asdf_plugin.download_prebuilt(input)))
}

// unpack_archive: Unpacks downloaded archives based on their file extension.
#[plugin_fn]
pub fn unpack_archive(Json(input): Json<UnpackArchiveInput>) -> FnResult<()> {
    let input_file = input.input_file;
    let output_dir = input.output_dir;

    // Need to ensure file type and unpack accordingly
    if input_file.ends_with(".tar.xz") {
        //TODO: Implement the untar and unzip moments
        // untar(input_file, output_dir)?;
    } else if input_file.ends_with(".zip") {
        // unzip(input_file, output_dir)?;
    } else {
        return Err(PluginError::UnsupportedArchiveFormat(format!(
            "Unsupported archive format: {}",
            input_file
        ))
        .into());
    }

    Ok(())
}

// detect_version_files: Specifies which files to check for version information.
#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    let asdf_plugin = AsdfPlugin {
        tool: Tool::default(),
    };
    Ok(Json(asdf_plugin.detect_version_files()))
}

// parse_version_file: Parses version information from specified files
#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let asdf_plugin = AsdfPlugin {
        tool: Tool::default(),
    };
    Ok(Json(asdf_plugin.parse_version_file(input)))
}

// locate_executables: Locates the installed tool's executable files.
#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        primary: Some(ExecutableConfig::new(
            env.os.for_native("bin/node", "node.exe"),
        )),
        globals_lookup_dirs: vec!["$DENO_INSTALL_ROOT/bin".into(), "$HOME/.deno/bin".into()],
        ..LocateExecutablesOutput::default()
    }))
}
