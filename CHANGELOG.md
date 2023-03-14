# Changelog

## Unreleased

#### 💥 Breaking

- When detecting a version and proto encounters a range/requirement using `^`, `~`, `>=`, etc, proto will now resolve the version against the currently installed versions in `~/.proto`, instead of resolving to an arbitray fixed version.

#### 🚀 Updates

- Added "bundled" as a supported alias for `npm`.
- Updated `node` to automatically install the `npm` version that comes bundled with Node.js.

#### 🐞 Fixes

- Another attempt to fix SSL issues.
