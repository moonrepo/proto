import type { OverrideProperties, SetRequired } from "type-fest";
import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import { reviveToolContext } from "../internal/helpers";
import type { ToolContext } from "../api";

/** Configuration for generated shim and symlinked binary files. */
export type ExecutableConfig = SetRequired<
  Partial<raw.ExecutableConfig>,
  "exe_path"
>;

/** Input passed to the `locate_executables` function. */
export type LocateExecutablesInput = OverrideProperties<
  raw.LocateExecutablesInput,
  {
    /** Current tool context. */
    context: ToolContext;
  }
>;

/** Output returned by the `locate_executables` function. */
export type LocateExecutablesOutput = OverrideProperties<
  Partial<raw.LocateExecutablesOutput>,
  {
    /**
     * Configures the primary/default executable to create.
     * If not provided, a primary shim and binary will *not* be created.
     */
    primary?: ExecutableConfig | null;

    /**
     * Configures secondary/additional executables to create.
     * The map key is the name of the shim/binary file.
     */
    secondary?: Record<string, ExecutableConfig>;
  }
>;

export const createLocateExecutables = createPluginFnFactory<
  LocateExecutablesInput,
  LocateExecutablesOutput
>({
  reviveInput: (input: raw.LocateExecutablesInput) => ({
    ...input,
    context: reviveToolContext(input.context),
  }),
});
