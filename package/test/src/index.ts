import {
  DetectVersionFilesOutput,
  DownloadPrebuiltInput,
  DownloadPrebuiltOutput,
  LoadVersionsInput,
  LoadVersionsOutput,
  LocateExecutablesInput,
  LocateExecutablesOutput,
  ParseVersionFileInput,
  ParseVersionFileOutput,
  RegisterToolInput,
  RegisterToolOutput,
  ResolveVersionInput,
  ResolveVersionOutput,
  UnresolvedVersionSpec,
  VerifyChecksumInput,
  VerifyChecksumOutput,
  VersionLike,
  VersionSpec,
  createDetectVersionFiles,
  createDownloadPrebuilt,
  createLoadVersions,
  createLocateExecutables,
  createParseVersionFile,
  createRegisterTool,
  createResolveVersion,
  createVerifyChecksum,
  fetchUrl,
  getHostEnvironment,
  getToolConfig,
  hostLog,
} from "@moonrepo/proto-pdk";

export const register_tool = createRegisterTool(
  (input: RegisterToolInput): RegisterToolOutput => {
    // TODO: hostLog is a bit awkward to use
    hostLog({ target: "stdout", message: `Registering tool: ${input.id}` });

    const config = getToolConfig();
    hostLog({ message: `Config = ${JSON.stringify(config)}` });

    return {
      name: "WASM Test",
      default_version: new UnresolvedVersionSpec.Alias("latest"),
      plugin_version: "1.0.0",
      type: "CLI",
      self_upgrade_commands: [],
    };
  }
);

// Detector

export const detect_version_files = createDetectVersionFiles(
  (): DetectVersionFilesOutput => {
    return {
      files: [".proto-wasm-version", ".protowasmrc"],
      ignore: ["node_modules"],
    };
  }
);

export const parse_version_file = createParseVersionFile(
  ({ content, file }: ParseVersionFileInput): ParseVersionFileOutput => {
    let version: UnresolvedVersionSpec | null = null;

    if (file === ".proto-wasm-version") {
      if (content.startsWith("version=")) {
        version = UnresolvedVersionSpec.parse(content.slice(8));
      }
    } else {
      version = UnresolvedVersionSpec.parse(content);
    }

    return {
      version,
    };
  }
);

// Downloader

export const download_prebuilt = createDownloadPrebuilt(
  ({ context }: DownloadPrebuiltInput): DownloadPrebuiltOutput => {
    const env = getHostEnvironment();
    const version = context.version;
    const arch = env.arch;

    let prefix: string;
    if (env.os === "linux") prefix = `node-v${version}-linux-${arch}`;
    else if (env.os === "macos") prefix = `node-v${version}-darwin-${arch}`;
    else if (env.os === "windows") prefix = `node-v${version}-win-${arch}`;
    else throw new Error("Not implemented");

    const filename =
      env.os === "windows" ? `${prefix}.zip` : `${prefix}.tar.xz`;

    return {
      archive_prefix: prefix,
      download_url: `https://nodejs.org/dist/v${version}/${filename}`,
      download_name: filename,
      checksum_url: `https://nodejs.org/dist/v${version}/SHASUMS256.txt`,
    };
  }
);

export const locate_executables = createLocateExecutables(
  (_: LocateExecutablesInput): LocateExecutablesOutput => {
    const env = getHostEnvironment();

    return {
      globals_lookup_dirs: ["$WASM_ROOT/bin", "$HOME/.wasm/bin"],
      primary: {
        exe_path: env.os === "windows" ? "node.exe" : "bin/node",
      },
      secondary: {
        global1: { exe_path: "bin/global1" },
      },
    };
  }
);

// Resolver

type NodeDistVersion = {
  version: `v${string}`;
};

export const load_versions = createLoadVersions(
  (_: LoadVersionsInput): LoadVersionsOutput => {
    const response = fetchUrl<NodeDistVersion[]>(
      "https://nodejs.org/dist/index.json"
    );

    const versions = response.map(
      (item) => new VersionSpec.Version(item.version.slice(1))
    );
    const latest = versions[0] ?? null;

    const aliases: Record<string, VersionLike> = {};
    if (latest) aliases["latest"] = latest;

    return {
      latest,
      versions,
      aliases,
    };
  }
);

export const resolve_version = createResolveVersion(
  ({ initial }: ResolveVersionInput): ResolveVersionOutput => {
    if (
      initial instanceof UnresolvedVersionSpec.Alias &&
      initial.alias === "node"
    ) {
      return {
        candidate: new UnresolvedVersionSpec.Alias("latest"),
      };
    }

    return {};
  }
);

// Verifier

export const verify_checksum = createVerifyChecksum(
  (_: VerifyChecksumInput): VerifyChecksumOutput => {
    // unfortunately the extism js pdk does not support filesystem access yet :(

    return {
      verified: true,
    };
  }
);
