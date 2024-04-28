import type { OverrideProperties } from "type-fest";
import type { ToolContext } from "../api";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import { reviveToolContext } from "../internal/helpers";
import type * as raw from "../internal/raw-api-types";

/**
 * Input passed to the `pre_install` and `post_install` hooks,
 * while a `proto install` command is running.
 */
export type InstallHookInput = OverrideProperties<
  raw.InstallHook,
  {
    /** Current tool context. */
    context: ToolContext;
  }
>;

export const createInstallHook = createPluginFnFactory<InstallHookInput, void>({
  reviveInput: (input: raw.InstallHook) => ({
    ...input,
    context: reviveToolContext(input.context),
  }),
});

export type PreInstallHookInput = InstallHookInput;
export const createPreInstallHook = createInstallHook;

export type PostInstallHookInput = InstallHookInput;
export const createPostInstallHook = createInstallHook;
