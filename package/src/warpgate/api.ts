import type * as raw from "../internal/raw-api-types";

/** Architecture of the host environment. */
export type HostArch = raw.SystemArch;

/** Libc being used in the host environment. */
export type HostLibc = raw.SystemLibc;

/** Operating system of the host environment. */
export type HostOS = raw.SystemOS;

export type {
  HostEnvironment,
  TestEnvironment,
} from "../internal/raw-api-types";
