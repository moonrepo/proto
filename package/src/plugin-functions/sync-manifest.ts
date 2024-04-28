import type * as raw from "../internal/raw-api-types";
import { createPluginFnFactory } from "../internal/create-plugin-function";
import { reviveToolContext } from "../internal/helpers";
import type { OverrideProperties } from "type-fest";
import type { VersionLike } from "../versions";
import type { ToolContext } from "../api";

/** Input passed to the `sync_manifest` function. */
export type SyncManifestInput = OverrideProperties<
  raw.SyncManifestInput,
  {
    /** Current tool context. */
    context: ToolContext;
  }
>;

/** Output returned by the `sync_manifest` function. */
export type SyncManifestOutput = OverrideProperties<
  raw.SyncManifestOutput,
  {
    /**
     * List of versions that are currently installed. Will replace
     * what is currently in the manifest.
     */
    versions?: VersionLike[] | null;
  }
>;

export const createSyncManifest = createPluginFnFactory<
  SyncManifestInput,
  SyncManifestOutput
>({
  reviveInput: (input: raw.SyncManifestInput) => ({
    ...input,
    context: reviveToolContext(input.context),
  }),
});
