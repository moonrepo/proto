import type { OverrideProperties } from "type-fest";
import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import { reviveToolContext } from "../internal/helpers";
import type { ToolContext } from "../api";

/** Input passed to the `native_uninstall` function. */
export type NativeUninstallInput = OverrideProperties<
  raw.NativeUninstallInput,
  {
    /** Current tool context. */
    context: ToolContext;
  }
>;

/** Output returned by the `native_uninstall` function. */
export type NativeUninstallOutput = raw.NativeUninstallOutput;

export const createNativeUninstall = createPluginFnFactory<
  NativeUninstallInput,
  NativeUninstallOutput
>({
  reviveInput: (input: raw.NativeUninstallInput) => ({
    ...input,
    context: reviveToolContext(input.context),
  }),
});
