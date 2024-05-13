import { SemVer } from "semver";
import type * as raw from "./raw-api-types";
import type { ToolContext } from "../api";
import { VersionSpec } from "../versions";

export function reviveToolContext(input: raw.ToolContext): ToolContext {
  return {
    ...input,
    proto_version: input.proto_version ? new SemVer(input.proto_version) : null,
    version: VersionSpec.parse(input.version),
  };
}
