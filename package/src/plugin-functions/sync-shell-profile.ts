import type { OverrideProperties } from "type-fest";
import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import { reviveToolContext } from "../internal/helpers";
import type { ToolContext } from "../api";

/** Input passed to the `sync_shell_profile` function. */
export type SyncShellProfileInput = OverrideProperties<
  raw.SyncShellProfileInput,
  {
    /** Current tool context. */
    context: ToolContext;
  }
>;

/** Output returned by the `sync_shell_profile` function. */
export type SyncShellProfileOutput = raw.SyncShellProfileOutput;

export const createSyncShellProfile = createPluginFnFactory<
  SyncShellProfileInput,
  SyncShellProfileOutput
>({
  reviveInput: (input: raw.SyncShellProfileInput) => ({
    ...input,
    context: reviveToolContext(input.context),
  }),
});
