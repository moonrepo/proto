import type * as raw from "./internal/raw-api-types";
import type { VersionSpec } from "./versions";
import type { SemVer } from "semver";
import type { OverrideProperties } from "type-fest";

/** Information about the current state of the tool. */
export type ToolContext = OverrideProperties<
  raw.ToolContext,
  {
    /** The version of proto (the core crate) calling plugin functions. */
    proto_version: SemVer | null;

    /** Current version. Will be a "latest" alias if not resolved. */
    version: VersionSpec;
  }
>;

export type { PluginType } from "./internal/raw-api-types";
