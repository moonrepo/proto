import type { OverrideProperties } from "type-fest";
import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import { UnresolvedVersionSpec, type VersionLike } from "../versions";

/** Input passed to the `load_versions` function. */
export type LoadVersionsInput = OverrideProperties<
  raw.LoadVersionsInput,
  {
    /** The alias or version currently being resolved. */
    initial: UnresolvedVersionSpec;
  }
>;

/** Output returned by the `load_versions` function. */
export type LoadVersionsOutput = OverrideProperties<
  raw.LoadVersionsOutput,
  {
    /** Mapping of aliases (channels, etc) to a version. */
    aliases?: Record<string, VersionLike>;

    /** Latest canary version. */
    canary?: VersionLike | null;

    /** Latest stable version. */
    latest?: VersionLike | null;

    /** List of available production versions to install. */
    versions: VersionLike[];
  }
>;

export const createLoadVersions = createPluginFnFactory<
  LoadVersionsInput,
  LoadVersionsOutput
>({
  reviveInput: (input: raw.LoadVersionsInput) => ({
    initial: UnresolvedVersionSpec.parse(input.initial),
  }),
});
