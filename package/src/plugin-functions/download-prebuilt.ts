import type { OverrideProperties, SetRequired } from "type-fest";
import type { ToolContext } from "../api";
import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import { reviveToolContext } from "../internal/helpers";

/** Input passed to the `download_prebuilt` function. */
export type DownloadPrebuiltInput = OverrideProperties<
  raw.DownloadPrebuiltInput,
  {
    /** Current tool context. */
    context: ToolContext;
  }
>;

/** Output returned by the `download_prebuilt` function. */
export type DownloadPrebuiltOutput = SetRequired<
  Partial<raw.DownloadPrebuiltOutput>,
  "download_url"
>;

export const createDownloadPrebuilt = createPluginFnFactory<
  DownloadPrebuiltInput,
  DownloadPrebuiltOutput
>({
  reviveInput: (input: raw.DownloadPrebuiltInput) => ({
    ...input,
    context: reviveToolContext(input.context),
  }),
});
