const aliasRegex = /[a-zA-Z][a-zA-Z0-9\-_/\.\*]/;

/**
 * Returns true if the provided value is an alias. An alias is a word that
 * maps to version, for example, "latest" -> "1.2.3".
 *
 * Is considered an alias if the string is alpha-numeric, starts with a
 * character, and supports `-`, `_`, `/`, `.`, and `*` characters.
 */
export function isAliasName(value: string): boolean {
  return aliasRegex.test(value);
}

/**
 * Cleans a potential version string by removing a leading `v` or `V`,
 * removing each occurence of `.*`, and removing invalid commas.
 */
export function cleanVersionString(value: string): string {
  let version = value.trim();

  if (version.includes("||")) {
    return version.split("||").map(cleanVersionString).join(" || ");
  }

  version = version.replace(/\.\*/g, "").replace(/&&/g, " ");

  if (version.startsWith("v") || version.startsWith("V")) {
    version = version.substring(1);
  }

  version = version.replace(/([><]=?)\s+(\d)/g, "$1$2");
  version = version.replace(/[, ]+/g, " ");

  return version;
}
