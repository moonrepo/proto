import { describe, test, expect } from "vitest";
import { isAliasName, cleanVersionString } from "./utils"; // Replace 'your_file_name' with the actual file name containing the functions

describe("isAliasName", () => {
  test("checks alias", () => {
    expect(isAliasName("foo")).toBe(true);
    expect(isAliasName("foo.bar")).toBe(true);
    expect(isAliasName("foo/bar")).toBe(true);
    expect(isAliasName("foo-bar")).toBe(true);
    expect(isAliasName("foo_bar-baz")).toBe(true);
    expect(isAliasName("alpha.1")).toBe(true);
    expect(isAliasName("beta-0")).toBe(true);
    expect(isAliasName("rc-1.2.3")).toBe(true);
    expect(isAliasName("next-2023")).toBe(true);

    expect(isAliasName("1.2.3")).toBe(false);
    expect(isAliasName("1.2")).toBe(false);
    expect(isAliasName("1")).toBe(false);
    expect(isAliasName("1-3")).toBe(false);
  });
});

describe("cleanVersionString", () => {
  test("cleans string", () => {
    expect(cleanVersionString("v1.2.3")).toBe("1.2.3");
    expect(cleanVersionString("V1.2.3")).toBe("1.2.3");

    expect(cleanVersionString("1.2.*")).toBe("1.2");
    expect(cleanVersionString("1.*.*")).toBe("1");
    expect(cleanVersionString("*")).toBe("*");

    expect(cleanVersionString(">= 1.2.3")).toBe(">=1.2.3");
    expect(cleanVersionString(">  1.2.3")).toBe(">1.2.3");
    expect(cleanVersionString("<1.2.3")).toBe("<1.2.3");
    expect(cleanVersionString("<=   1.2.3")).toBe("<=1.2.3");

    expect(cleanVersionString("1.2, 3")).toBe("1.2 3");
    expect(cleanVersionString("1,3, 4")).toBe("1 3 4");
    expect(cleanVersionString("1,2")).toBe("1 2");
    expect(cleanVersionString("1 && 2")).toBe("1 2");
  });

  test("handles commas", () => {
    expect(cleanVersionString("1,2")).toBe("1 2");
    expect(cleanVersionString("1  2")).toBe("1 2");
    expect(cleanVersionString("1   2")).toBe("1 2");
    expect(cleanVersionString("1,2")).toBe("1 2");
    expect(cleanVersionString("1 ,2")).toBe("1 2");
    expect(cleanVersionString("1, 2")).toBe("1 2");
    expect(cleanVersionString("1 , 2")).toBe("1 2");
    expect(cleanVersionString("1  , 2")).toBe("1 2");
    expect(cleanVersionString("1,  2")).toBe("1 2");
  });
});
