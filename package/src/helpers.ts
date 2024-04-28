import { UnsupportedOSError, UnsupportedTargetError } from "./errors";
import type { HostArch, HostEnvironment, HostOS } from "./warpgate/api";

type Permutations = Partial<Record<HostOS, HostArch[]>>;

/**
 * Validate the current host OS and architecture against the
 * supported list of target permutations.
 */
export function checkSupportedOsAndArch(
  tool: string,
  { os, arch }: HostEnvironment,
  permutations: Permutations
) {
  const archs = permutations[os];

  if (!archs) {
    throw new UnsupportedOSError({ tool, os });
  }

  if (!archs.includes(arch)) {
    throw new UnsupportedTargetError({ tool, arch, os });
  }
}

/**
 * Return a Rust target triple for the current host OS and architecture.
 */
export function getTargetTriple(
  { os, arch, libc }: HostEnvironment,
  tool: string
): string {
  switch (os) {
    case "linux":
      return `${toRustArch(arch)}-unknown-linux-${
        libc === "musl" ? "musl" : "gnu"
      }`;
    case "macos":
      return `${toRustArch(arch)}-apple-darwin`;
    case "windows":
      return `${toRustArch(arch)}-pc-windows-msvc`;
    default:
      throw new UnsupportedTargetError({ tool, os, arch });
  }
}

/**
 * Get proto tool configuration that was configured in a `.prototools` file.
 */
export function getToolConfig(
  defaults: Record<string, unknown> = {}
): Record<string, unknown> {
  const json = Config.get("proto_tool_config");

  if (!json) return defaults;

  try {
    return {
      ...defaults,
      ...JSON.parse(json),
    };
  } catch {
    console.error("Unable to parse tool config");
    return defaults;
  }
}

/**
 * Convert to a [`std::env::costs::ARCH`] compatible string.
 */
export function toRustArch(arch: HostArch): string {
  switch (arch) {
    case "x64":
      return "x86_64";
    case "arm64":
      return "aarch64";
    case "longarm64":
      return "loongarch64";
    default:
      return arch;
  }
}
