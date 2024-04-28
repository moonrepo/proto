import { test, expect } from "vitest";
import { UnresolvedVersionSpec } from "./unresolved-spec";

expect.addEqualityTesters([
  (a, b) => a instanceof UnresolvedVersionSpec && a.equals(b),
]);

test("canary", () => {
  expect(UnresolvedVersionSpec.parse("canary")).toEqual(
    new UnresolvedVersionSpec.Canary()
  );
});

test("aliases", () => {
  expect(UnresolvedVersionSpec.parse("latest")).toEqual(
    new UnresolvedVersionSpec.Alias("latest")
  );
  expect(UnresolvedVersionSpec.parse("stable")).toEqual(
    new UnresolvedVersionSpec.Alias("stable")
  );
  expect(UnresolvedVersionSpec.parse("legacy-2023")).toEqual(
    new UnresolvedVersionSpec.Alias("legacy-2023")
  );
  expect(UnresolvedVersionSpec.parse("future/202x")).toEqual(
    new UnresolvedVersionSpec.Alias("future/202x")
  );
});

test("versions", () => {
  expect(UnresolvedVersionSpec.parse("v1.2.3")).toEqual(
    new UnresolvedVersionSpec.Version("1.2.3")
  );
  expect(UnresolvedVersionSpec.parse("1.2.3")).toEqual(
    new UnresolvedVersionSpec.Version("1.2.3")
  );
});

test("requirements", () => {
  expect(UnresolvedVersionSpec.parse("1.2")).toEqual(
    new UnresolvedVersionSpec.Req("~1.2")
  );
  expect(UnresolvedVersionSpec.parse("1")).toEqual(
    new UnresolvedVersionSpec.Req("~1")
  );
  expect(UnresolvedVersionSpec.parse("1.2.*")).toEqual(
    new UnresolvedVersionSpec.Req("~1.2")
  );
  expect(UnresolvedVersionSpec.parse("1.*")).toEqual(
    new UnresolvedVersionSpec.Req("~1")
  );
  expect(UnresolvedVersionSpec.parse(">1")).toEqual(
    new UnresolvedVersionSpec.Req(">1")
  );
  expect(UnresolvedVersionSpec.parse("<=1")).toEqual(
    new UnresolvedVersionSpec.Req("<=1")
  );
  expect(UnresolvedVersionSpec.parse("1, 2")).toEqual(
    new UnresolvedVersionSpec.Req("1 2")
  );
  expect(UnresolvedVersionSpec.parse("1,2")).toEqual(
    new UnresolvedVersionSpec.Req("1 2")
  );
  expect(UnresolvedVersionSpec.parse("1 2")).toEqual(
    new UnresolvedVersionSpec.Req("1 2")
  );
});

test("any requirements", () => {
  expect(UnresolvedVersionSpec.parse("^1.2 || ~1 || 3,4")).toEqual(
    new UnresolvedVersionSpec.ReqAny("~1 || ^1.2 || 3 4")
  );
});
