import type { OverrideProperties } from "type-fest";
import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import type { UnresolvedVersionLike } from "../versions";

/** Controls aspects of the tool inventory. */
export type ToolInventoryMetadata = Partial<raw.ToolInventoryMetadata>;

/** Input passed to the `register_tool` function. */
export type ToolMetadataInput = raw.ToolMetadataInput;

/** Output returned by the `register_tool` function. */
export type ToolMetadataOutput = OverrideProperties<
  raw.ToolMetadataOutput,
  {
    /** Controls aspects of the tool inventory. */
    inventory?: ToolInventoryMetadata;

    /** Default alias or version to use as a fallback. */
    default_version?: UnresolvedVersionLike | null;
  }
>;

export const createRegisterTool = createPluginFnFactory<
  ToolMetadataInput,
  ToolMetadataOutput
>();

export type RegisterToolInput = ToolMetadataInput;
export type RegisterToolOutput = ToolMetadataOutput;
