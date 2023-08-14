# Changelog

## Unreleased

#### 🐞 Fixes

- Fixed an issue where `proto setup` would sometimes not initialize.

## 0.14.0

#### 💥 Breaking

- Versions defined in `.prototools` must be fully-qualified semantic versions. Partial versions (missing patch or minor) are no longer supported. This may change in the future based on feedback.
- Tool and plugin names must now be in kebab-case.
- Node
  - Yarn 2+ is now installed using `@yarnpkg/cli-dist`. We no longer downgrade to the latest v1.
  - Will no longer detect a version from `engines` in `package.json` (too problematic).
- TOML API
  - Moved `install.global-args` to `globals.install-args`.
  - Moved `install.globals-dir` to `globals.lookup-dirs`.
  - Removed `install.unpack` (proto should be smart enough now to figure this out).
- WASM API
  - Renamed host function `trace` to `host_log`.

#### 🚀 Updates

- [Node.js (and package managers)](https://github.com/moonrepo/node-plugin) and [Rust](https://github.com/moonrepo/rust-plugin) are now powered by WASM plugins, and have been removed from core.
  - Please report any issues you encounter or discrepancies from the previous release!
- Improved version detection and requirement resolution.
- Rust
  - Will now detect a version/channel from `rust-toolchain.toml` and `rust-toolchain`.
- TOML API
  - Added new `globals` section.
- WASM API
  - Added `host_log!` and `exec_command!` macros for working with host functions.
  - Added `default_version` and `inventory` fields to `ToolMetadataOutput`.
  - Added `home_dir` field to `ToolMetadataInput` and `LocateBinsInput`.
  - Added `globals_prefix` field to `LocateBinsOutput`.
  - Updated `exec_command` with stream/inherit support.
  - Updated `bin_path` in `LocateBinsOutput` and `ShimConfig` to a `PathBuf`.

## 0.13.1

#### 🚀 Updates

- WASM API
  - Added the plugin `id` to every `env` field.
  - Added `initial` to `LoadVersionsInput`.

#### 🐞 Fixes

- Fixed an issue where version requirements using `>`, `>=`, `<`, or `<=` wouldn't parse correctly.

## 0.13.0

#### 🚀 Updates

- [Bun](https://github.com/moonrepo/bun-plugin), [Deno](https://github.com/moonrepo/deno-plugin), and [Go](https://github.com/moonrepo/go-plugin) are now powered by WASM plugins, and have been removed from core.
  - This is an experiment before we migrate Node.js and Rust to WASM.
  - Please report any issues you encounter or discrepancies from the previous release!
- Added a new plugin configuration locator, `github:org/repo`.
- Added a `proto plugins` command, for listing all active and configured plugins.
- Updated `proto clean` and `proto use` to load and merge all `.prototools` from the current directory and upwards.
- WASM API
  - Added a `format_bin_name` function.
  - Added a `check_supported_os_and_arch` function.
  - Added a `checksum` field to `VerifyChecksumInput`.
  - Renamed `ParseVersion*` to `ParseVersionFile*`.
  - Published a `proto_pdk_test_utils` package for writing tests.

#### ⚙️ Internal

- Rewrote the plugin downloading and registry layers.
- Updated Rust to v1.71.

## 0.12.1

#### 🚀 Updates

- WASM API
  - Added a `trace` host function, for logging trace messages via the host machine.
  - Added an `exec_command` host function, for executing system commands on the host machine.
  - Added an `fetch_url_with_cache` function, for making an HTTP request and caching the response.
  - Added `fallback_last_globals_dir` field to `LocateBinsOutput`.
  - Updated `load_git_tags` to use the new `exec_command` function.

#### 🐞 Fixes

- Fixed an issue where shims were always being created.

## 0.12.0

#### 💥 Breaking

- Removed `schema:` as a prefix for TOML based plugins. Use `source:` instead.

#### 🚀 Updates

- Added experimental support for WASM based plugins.
- Added `source:` as a plugin prefix.

#### ⚙️ Internal

- Updated Cargo dependencies.
- Updated to `cargo-dist` v0.0.6.

## 0.11.2

#### 🐞 Fixes

- Fixed an args escaping issue for Unix based shims.

## 0.11.1

#### 🐞 Fixes

- Fixed an issue where `--log` would fail with an invalid value when running a tool.

## 0.11.0

#### 🚀 Updates

- Added shim support for `bunx` (bun), `pnpx` (pnpm), and `yarnpkg` (yarn).
- Added a global `--log` option to all commands.
- Improved tracing log messages.

#### ⚙️ Internal

- Updated Cargo dependencies.
- Refactored shims from the ground up for easier maintenance.

## 0.10.6

#### 🚀 Updates

- Added `PROTO_CACHE` environment variable to control whether to read from temporary cache or not.

## 0.10.5

#### 🚀 Updates

- Added `PROTO_AUTO_CLEAN`, `PROTO_AUTO_INSTALL`, and `PROTO_NODE_INTERCEPT_GLOBALS` environment variables.
- Added a `node-intercept-globals` setting to control the Node.js/npm/etc globals behavior.

## 0.10.4

#### 🚀 Updates

- Updated Node.js to use x64 on arm64 machines when arm64 is not available (<16).

#### 🐞 Fixes

- Fixed an issue where writing to shells would omit a trailing newline.

## 0.10.3

#### 🚀 Updates

- Improved error messages for missing or unsupported downloads.

#### 🐞 Fixes

- Fixed an issue where `proto upgrade` would fail on Windows.
- Fixed an issue where version requirement `>=0.0.0 <0.0.0` would fail to parse.

## 0.10.2

#### 🐞 Fixes

- Fixed a bad release.

## 0.10.1

#### 🐞 Fixes

- Fixed an issue where `proto install-global` for Node.js would recursively call and fail.

## 0.10.0

#### 💥 Breaking

- Updated Windows `~/.proto/bin` shims to use `.cmd` files instead of `.ps1` files. This will interop better with the default `PATHEXT` environment variable.

## 0.9.2

#### 🐞 Fixes

- Fixed an index out of bounds issue with `yarn`.
- Fixed an issue with Windows shims not being ran correctly.
- An attempt to fix "proto killed" errors after running `proto upgrade`.

## 0.9.1

#### 🚀 Updates

- Updated npm/pnpm/yarn to error when attempting to install a global binary. Use `proto install-global` instead.

#### ⚙️ Internal

- Improved handling of alternate tool binaries, like `npx` and `node-gyp`.

## 0.9.0

#### 🚀 Updates

- Added `install.unpack` setting to TOML plugin schema.
- Updated `npm` to also create a `node-gyp` global shim.

#### ⚙️ Internal

- Updated Cargo dependencies.

## 0.8.3

#### 🐞 Fixes

- Fixed an issue where shim files would sometimes not be found.

#### ⚙️ Internal

- Updated Cargo dependencies.
- Updated to `cargo-dist` v0.0.6.

## 0.8.2

#### 🐞 Fixes

##### Rust

- Fixed an issue where "is installed" checks would sometimes fail.
- Fixed an issue where it would load the manifest cache from the wrong path.

#### ⚙️ Internal

- Migrated to a new sandbox implementation for testing.

## 0.8.1

#### 🐞 Fixes

- Installed versions are now sorted during detection, so that latest versions are always used first.
- Updated `proto clean` to properly handle cleaning installed plugins.

## 0.8.0

#### 🚀 Updates

- Improved version detection to scan locally installed versions more often, instead of resolving to the latest remote version.
  - This will result in far less "version not installed" errors.
  - Fully-qualified semantic versions are still used as-is.
- Updated `proto use` to also install tools based on environment/ecosystem config in the current working directory.
  - For example, will install a `packageManager` from `package.json`.
  - This is pseudo replacement for `corepack`.
- Updated shims to only be created on initial install, or when the internal API changes, instead of always.

#### 🐞 Fixes

- Fixed an issue with `npx` not handling args correctly.
- Fixed an issue where `moon clean` would fail on an empty/missing plugin.

## 0.7.2

#### 🐞 Fixes

- Temporary fix for "text file busy" error when creating shims.

## 0.7.1

#### 🐞 Fixes

- Attempt to fix an issue where `manifest.json` would fail to parse while running concurrent processes.

## 0.7.0

#### 🚀 Updates

- Added TOML schema based plugins support, allowing for custom tools/CLIs to be managed in proto.
- Added a `[plugins]` section to `~/.proto/config.toml` and `.prototools`.
- Added a `--yes` option to `proto clean`, allowing prompts to be bypassed.
- Added a `auto-clean` setting to `~/.proto/config.toml`, enabling automatic cleaning when `proto use` is ran.
- Updated `proto use` to also install configured plugins.

#### ⚙️ Internal

- Updated Rust to v1.69.
- Updated tool and plugin names to be kebab-case.

## 0.6.1

#### ⚙️ Internal

- Added read/write file locking for the `manifest.json` file.

## 0.6.0

#### 🚀 Updates

- Added a `proto clean` command for removing old/stale tool installations.
- Added a `proto list-global` command for listing all installed global packages for a tool.
- Updated `proto install-global` to support installing multiple globals.

#### ⚙️ Internal

- Greatly improved error messages.
- We now track install/last used timestamps for future functionality.

## 0.5.0

#### 🚀 Updates

- Added a `proto install-global` command for installing global packages for a tool.
- Added `proto alias` and `proto unalias` commands for creating custom version aliases.

#### 🐞 Fixes

- Fixed an issue where `PROTO_LOG` logs were not always shown.

#### ⚙️ Internal

- Updated cargo dependencies.

## 0.4.0

#### 🚀 Updates

- Added Rust as a supported language.
  - Requires `rustup` to be installed globally.
- Added a global user config at `~/.proto/config.toml`.
  - Added a new setting `auto-install`, that will automatically install a missing tool when `proto run` is executed.
- Added a `proto upgrade` command for upgrading the proto binary to latest.
- Added spinners and progress bars to install, uninstall, and upgrade flows.
- Updated Node.js to download `.tar.xz` archives, resulting in smaller files and less bandwidth.

#### 🐞 Fixes

- Updated `proto setup` on Windows to use the Windows registry when updating `PATH`.

#### ⚙️ Internal

- Added support for `.tar.xz` archives.

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
