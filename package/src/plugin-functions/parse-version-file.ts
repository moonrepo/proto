import type { OverrideProperties } from "type-fest";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import type * as raw from "../internal/raw-api-types";
import type { UnresolvedVersionLike } from "../versions";

/** Input passed to the `parse_version_file` function. */
export type ParseVersionFileInput = raw.ParseVersionFileInput;

/** Output returned by the `parse_version_file` function. */
export type ParseVersionFileOutput = OverrideProperties<
  raw.ParseVersionFileOutput,
  {
    /**
     * The version that was extracted from the file.
     * Can be a semantic version or a version requirement/range.
     */
    version?: UnresolvedVersionLike | null;
  }
>;

export const createParseVersionFile = createPluginFnFactory<
  ParseVersionFileInput,
  ParseVersionFileOutput
>();
