import type { OverrideProperties } from "type-fest";
import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import { reviveToolContext } from "../internal/helpers";
import type { ToolContext } from "../api";

/** Input passed to the `unpack_archive` function. */
export type UnpackArchiveInput = OverrideProperties<
  raw.UnpackArchiveInput,
  {
    /** Current tool context. */
    context: ToolContext;
  }
>;

export const createUnpackArchive = createPluginFnFactory<
  UnpackArchiveInput,
  void
>({
  reviveInput: (input: raw.UnpackArchiveInput) => ({
    ...input,
    context: reviveToolContext(input.context),
  }),
});
