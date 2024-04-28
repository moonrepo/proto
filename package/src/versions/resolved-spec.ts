import { SemVer } from "semver";
import { cleanVersionString, isAliasName } from "./utils";
import { UnresolvedVersionSpec } from "./unresolved-spec";

// TODO: Maybe make the SemVer library an internal detail and hide it as it
// would be nice to later switch it for a tree-shakeable alternative.

/**
 * Represents a resolved version or alias.
 */
export abstract class VersionSpec {
  abstract toUnresolvedSpec(): UnresolvedVersionSpec;
  abstract toString(): string;

  toJSON() {
    return this.toString();
  }

  equals(other: { toString(): string }): boolean {
    return this.toString() === other.toString();
  }

  /**
   * Parse the provided string into a resolved specification based
   * on the following rules, in order:
   *
   * - If the value "canary", map as `Canary` variant.
   * - If an alpha-numeric value that starts with a character, map as `Alias`.
   * - Else parse with [`Version`], and map as `Version`.
   */
  static parse(value: string): VersionSpec {
    if (value == "canary") {
      return new VersionSpec.Canary();
    }

    value = cleanVersionString(value);

    if (isAliasName(value)) {
      return new VersionSpec.Alias(value);
    }

    return new VersionSpec.Version(value);
  }
}

export namespace VersionSpec {
  /**
   * An alias that is used as a map to a version.
   */
  export class Alias extends VersionSpec {
    constructor(public readonly alias: string) {
      super();
    }

    override toString(): string {
      return this.alias;
    }

    override toUnresolvedSpec(): UnresolvedVersionSpec {
      return new UnresolvedVersionSpec.Alias(this.alias);
    }
  }

  /**
   * A special canary target.
   */
  export class Canary extends Alias {
    constructor() {
      super("canary");
    }
  }

  /**
   * A fully-qualified semantic version.
   */
  export class Version extends VersionSpec {
    public readonly version: SemVer;

    constructor(version: SemVer | string) {
      super();

      this.version = new SemVer(version);
    }

    override toString(): string {
      return this.version.toString();
    }

    override toUnresolvedSpec(): UnresolvedVersionSpec {
      return new UnresolvedVersionSpec.Version(this.version);
    }
  }
}
