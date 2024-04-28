import { type Comparator, Range, SemVer } from "semver";
import { cleanVersionString, isAliasName } from "./utils";
import { VersionSpec } from "./resolved-spec";
import { PluginError } from "../errors";

/**
 * Represents an unresolved version or alias that must be resolved
 * to a fully-qualified and semantic result.
 */
export abstract class UnresolvedVersionSpec {
  /**
   * Convert the current unresolved specification to a resolved specification.
   * Note that this *does not* actually resolve or validate against a manifest,
   * and instead simply constructs the [`VersionSpec`].
   *
   * Furthermore, the `Req` and `ReqAny` variants will throw an error, as they
   * are not resolved or valid versions.
   */
  abstract toResolvedSpec(): VersionSpec;

  abstract toString(): string;

  toJSON() {
    return this.toString();
  }

  equals(other: { toString(): string }): boolean {
    return this.toString() === other.toString();
  }

  /**
   * Parse the provided string into an unresolved specification based
   * on the following rules, in order:
   *
   * - If the value "canary", map as `Canary` variant.
   * - If an alpha-numeric value that starts with a character, map as `Alias`.
   * - If contains `||`, split and parse each item with [`VersionReq`],
   *   and map as `ReqAny`.
   * - If contains `,` or ` ` (space), parse with [`VersionReq`], and map as `Req`.
   * - If starts with `=`, `^`, `~`, `>`, `<`, or `*`, parse with [`VersionReq`],
   *   and map as `Req`.
   * - Else parse with [`Version`], and map as `Version`.
   */
  static parse(value: string): UnresolvedVersionSpec {
    if (value == "canary") {
      return new UnresolvedVersionSpec.Canary();
    }

    value = cleanVersionString(value);

    if (isAliasName(value)) {
      return new UnresolvedVersionSpec.Alias(value);
    }

    // OR requirements (Node.js)
    if (value.includes("||")) {
      return new UnresolvedVersionSpec.ReqAny(value);
    }

    // AND requirements
    if (value.includes(",")) {
      return new UnresolvedVersionSpec.Req(value);
    }

    if (["=", "^", "~", ">", "<", "*"].includes(value.charAt(0))) {
      return new UnresolvedVersionSpec.Req(value);
    }

    const dotCount = [...value].reduce(
      (dots, char) => (char === "." ? dots + 1 : dots),
      0
    );

    if (dotCount < 2) {
      return new UnresolvedVersionSpec.Req(`~${value}`);
    }

    return new UnresolvedVersionSpec.Version(value);
  }
}

export namespace UnresolvedVersionSpec {
  /**
   * An alias that is used as a map to a version.
   */
  export class Alias extends UnresolvedVersionSpec {
    constructor(public readonly alias: string) {
      super();
    }

    override toResolvedSpec(): VersionSpec {
      return new VersionSpec.Alias(this.alias);
    }

    override toString(): string {
      return this.alias;
    }
  }

  /**
   * A special canary target.
   */
  export class Canary extends Alias {
    constructor() {
      super("canary");
    }

    override toResolvedSpec(): VersionSpec {
      return new VersionSpec.Canary();
    }
  }

  /**
   * A fully-qualified semantic version.
   */
  export class Version extends UnresolvedVersionSpec {
    public readonly version: SemVer;

    constructor(version: SemVer | string) {
      super();

      this.version = new SemVer(version);
    }

    override toResolvedSpec(): VersionSpec {
      return new VersionSpec.Version(this.version);
    }

    override toString(): string {
      return this.version.toString();
    }
  }

  /**
   * A list of requirements to match any against (joined by `||`).
   */
  export class ReqAny extends UnresolvedVersionSpec {
    public readonly range: Range;

    constructor(range: Range | string) {
      super();

      // This is pretty nasty, but we presort the ranges and comparators,
      // so the equality check is just a string comparison.
      const unsortedRange = new Range(range);
      const sortedRaw = unsortedRange.set
        .map((comparators) => [...comparators].sort().join(" "))
        .sort()
        .join("||");

      this.range = new Range(sortedRaw);
    }

    override toResolvedSpec(): VersionSpec {
      throw new PluginError("invalid operation");
    }

    override toString(): string {
      return this.range.range;
    }
  }

  /**
   * A partial version, requirement, or range (`^`, `~`, etc).
   */
  export class Req extends ReqAny {
    get comparators() {
      return this.range.set[0];
    }

    constructor(...comparators: readonly (Comparator | string)[]) {
      super(comparators.map((c) => c.toString()).join(" "));
    }
  }
}
