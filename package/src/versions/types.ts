import type { SemVer } from "semver";
import type { VersionSpec } from "./resolved-spec";
import type { UnresolvedVersionSpec } from "./unresolved-spec";

// TODO: should we allow passing raw strings as version-likes?
export type VersionLike =
  | VersionSpec.Version
  | UnresolvedVersionSpec.Version
  | SemVer;

export type ResolvedVersionLike =
  | VersionSpec
  | VersionLike
  | UnresolvedVersionSpec.Alias
  | UnresolvedVersionSpec.Canary;

export type UnresolvedVersionLike = UnresolvedVersionSpec | ResolvedVersionLike;
