# Changelog

## Unreleased

#### 🚀 Updates

- Added a global user config at `~/.proto/config.toml`.
  - Added a new setting `auto-install`, that will automatically install a missing tool when `proto run` is executed.

#### 🐞 Fixes

- Updated `proto setup` on Windows to use the Windows registry when updating `PATH`.

## 0.3.2

#### 🐞 Fixes

- Fixed `proto setup` to fallback to a shell instead of failing.

## 0.3.1

#### 🐞 Fixes

- Fixed the `npx` shim not being created.
- Fixed Windows installation issues.

## 0.3.0

#### 💥 Breaking

- When detecting a version and proto encounters a range/requirement using `^`, `~`, `>=`, etc, proto will now resolve the version against the currently installed versions in `~/.proto`, instead of resolving to an arbitray fixed version.

#### 🚀 Updates

- Added "bundled" as a supported alias for `npm`.
- Updated `proto local` and `proto global` to support aliases as well as versions.
- Updated `go` to automatically set `GOBIN` in your shell profile if has not been.
- Updated `node` to automatically install the `npm` version that comes bundled with Node.js.

#### 🐞 Fixes

- Another attempt to fix SSL issues.
