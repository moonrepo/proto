import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import { reviveToolContext } from "../internal/helpers";
import type { ToolContext } from "../api";
import type { OverrideProperties } from "type-fest";

/** Input passed to the `native_install` function. */
export type NativeInstallInput = OverrideProperties<
  raw.NativeInstallInput,
  {
    /** Current tool context. */
    context: ToolContext;
  }
>;

/** Output returned by the `native_install` function. */
export type NativeInstallOutput = raw.NativeInstallOutput;

export const createNativeInstall = createPluginFnFactory<
  NativeInstallInput,
  NativeInstallOutput
>({
  reviveInput: (input: raw.NativeInstallInput) => ({
    ...input,
    context: reviveToolContext(input.context),
  }),
});
