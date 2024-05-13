import { test, expect } from "vitest";
import { VersionSpec } from "./resolved-spec";

expect.addEqualityTesters([(a, b) => a instanceof VersionSpec && a.equals(b)]);

test("canary", () => {
  expect(VersionSpec.parse("canary")).toEqual(new VersionSpec.Canary());
});

test("alias", () => {
  expect(VersionSpec.parse("latest")).toEqual(new VersionSpec.Alias("latest"));
  expect(VersionSpec.parse("stable")).toEqual(new VersionSpec.Alias("stable"));
  expect(VersionSpec.parse("legacy-2023")).toEqual(
    new VersionSpec.Alias("legacy-2023")
  );
  expect(VersionSpec.parse("future/202x")).toEqual(
    new VersionSpec.Alias("future/202x")
  );
});

test("versions", () => {
  expect(VersionSpec.parse("v1.2.3")).toEqual(new VersionSpec.Version("1.2.3"));
  expect(VersionSpec.parse("1.2.3")).toEqual(new VersionSpec.Version("1.2.3"));
});

test("error when missing patch", () => {
  expect(() => {
    VersionSpec.parse("1.2");
  }).toThrowError("Invalid Version: 1");
});

test("error when missing minor", () => {
  expect(() => {
    VersionSpec.parse("1");
  }).toThrowError("Invalid Version: 1");
});

test("error when invalid char", () => {
  expect(() => {
    VersionSpec.parse("%");
  }).toThrowError("Invalid Version: %");
});
