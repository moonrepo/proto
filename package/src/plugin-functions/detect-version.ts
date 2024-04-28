import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";

/** Output returned by the `detect_version_files` function. */
export type DetectVersionFilesOutput = raw.DetectVersionOutput;

export const createDetectVersionFiles = createPluginFnFactory<
  never,
  DetectVersionFilesOutput
>();
