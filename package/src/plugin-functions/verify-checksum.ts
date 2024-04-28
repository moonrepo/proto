import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import { reviveToolContext } from "../internal/helpers";
import type { ToolContext } from "../api";
import type { OverrideProperties } from "type-fest";

/** Output returned by the `verify_checksum` function. */
export type VerifyChecksumInput = OverrideProperties<
  raw.VerifyChecksumInput,
  {
    /** Current tool context. */
    context: ToolContext;
  }
>;

/** Output returned by the `verify_checksum` function. */
export type VerifyChecksumOutput = raw.VerifyChecksumOutput;

export const createVerifyChecksum = createPluginFnFactory<
  VerifyChecksumInput,
  VerifyChecksumOutput
>({
  reviveInput: (input: raw.VerifyChecksumInput) => ({
    ...input,
    context: reviveToolContext(input.context),
  }),
});
