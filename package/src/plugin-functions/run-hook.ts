import type { OverrideProperties } from "type-fest";
import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import { reviveToolContext } from "../internal/helpers";
import type { ToolContext } from "../api";

/**
 * Input passed to the `pre_run` hook, before a `proto run` command
 * or language binary is ran.
 */
export type RunHookInput = OverrideProperties<
  raw.RunHook,
  {
    /** Current tool context. */
    context: ToolContext;
  }
>;

/** Output returned from the `pre_run` hook. */
export type RunHookOutput = raw.RunHookResult;

export const createRunHook = createPluginFnFactory<RunHookInput, RunHookOutput>(
  {
    reviveInput: (input: raw.RunHook) => ({
      ...input,
      context: reviveToolContext(input.context),
    }),
  }
);

export type PreRunHookInput = RunHookInput;
export type PreRunHookOutput = RunHookOutput;
export const createPreRunHook = createRunHook;
