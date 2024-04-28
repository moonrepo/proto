import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import {
  UnresolvedVersionSpec,
  type UnresolvedVersionLike,
  type VersionLike,
} from "../versions";
import type { OverrideProperties } from "type-fest";

/** Input passed to the `resolve_version` function. */
export type ResolveVersionInput = OverrideProperties<
  raw.ResolveVersionInput,
  {
    /** The alias or version currently being resolved. */
    initial: UnresolvedVersionSpec;
  }
>;

/** Output returned by the `resolve_version` function. */
export type ResolveVersionOutput = OverrideProperties<
  raw.ResolveVersionOutput,
  {
    /** New alias or version candidate to resolve. */
    candidate?: UnresolvedVersionLike | null;

    /**
     * An explicitly resolved version to be used as-is.
     * Note: Only use this field if you know what you're doing!
     */
    version?: VersionLike | null;
  }
>;

export const createResolveVersion = createPluginFnFactory<
  ResolveVersionInput,
  ResolveVersionOutput
>({
  reviveInput: (input: raw.ResolveVersionInput) => ({
    ...input,
    initial: UnresolvedVersionSpec.parse(input.initial),
  }),
});
