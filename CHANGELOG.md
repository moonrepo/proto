# Changelog

## Plugin changelogs

- [Bun](https://github.com/moonrepo/bun-plugin/blob/master/CHANGELOG.md)
- [Deno](https://github.com/moonrepo/deno-plugin/blob/master/CHANGELOG.md)
- [Go](https://github.com/moonrepo/go-plugin/blob/master/CHANGELOG.md)
- [Node](https://github.com/moonrepo/node-plugin/blob/master/CHANGELOG.md)
- [Python](https://github.com/moonrepo/python-plugin/blob/master/CHANGELOG.md)
- [Rust](https://github.com/moonrepo/rust-plugin/blob/master/CHANGELOG.md)
- [Schema](https://github.com/moonrepo/schema-plugin/blob/master/CHANGELOG.md)

## Unreleased

#### 💥 Breaking

> To ease the migration process, we've added a new migrate command. Simply run `proto migrate v0.20` after upgrading proto!

- The generated shims have moved to `~/.proto/shims` from `~/.proto/bin`. You'll need to manually update `PATH` in your shell profile if you'd like to continue using the runtime version detection functionality.

  ```diff
  export PROTO_HOME="$HOME/.proto"
  -export PATH="$PROTO_HOME/bin:$PATH"
  +export PATH="$PROTO_HOME/shims:$PROTO_HOME/bin:$PATH"
  ```

  Furthermore, we suggest deleting all files in `~/.proto/bin` except for `proto(.exe)`.

#### 🚀 Updates

- Reworked the `~/.proto/bin` directory to now contain symlinks to the original tool executables. This is a non-shim based alternative that can be used stand-alone or in unison with our shims.
  - The globally pinned version is the version that's symlinked. This can be updated with `proto install --pin`.
  - This approach _does not_ detect a version at runtime.
- Added a `proto migrate` command for easily applying changes between breaking releases.
- Added support for minisign checksum files. Can now verify `.minisig` signatures for downloaded tools.
- Updated `proto use` to install tools in parallel.
- Updated `proto plugins` and `proto tools` to load plugins in parallel.
- TOML API
  - Added an `install.checksum_public_key` setting.
- WASM API
  - Added a `checksum_public_key` field to `DownloadPrebuiltOutput`.
  - Removed `checksum` from `VerifyChecksumInput`.

#### ⚙️ Internal

- Minor performance improvements to runtime version detection.

## 0.19.3

#### 🚀 Updates

- Ensures the installation directory is empty before unpacking/moving files during an install.
- WASM API
  - Added `install_dir` to `DownloadPrebuiltInput` and `NativeInstallInput`.

#### ⚙️ Internal

- Updated dependencies.

## 0.19.2

#### 🚀 Updates

- Updated `proto clean` to also clean the `~/.proto/temp` directory.
- Updated `proto install` to unpack installs to a temporary directory, before moving to the final store location.

## 0.19.1

#### 🚀 Updates

- The file loaded for `proto outdated` is now output in the terminal.
- WASM API
  - Added `get_env_var` and `set_env_var` host functions.
  - Added `host_env!` macro.

## 0.19.0

#### 💥 Breaking

- Removed `proto global`, use `proto pin --global` instead.
- Removed `proto local`, use `proto pin` instead.

#### 🚀 Updates

- Added a `proto outdated` command that'll check for new versions of configured tools.
- Added a `proto pin` command, which is a merge of the old `proto global` and `proto local` commands.
- Added a `pin-latest` setting to `~/.proto/config.toml` that'll automatically pin tools when they're being installed with the "latest" version.
- Updated `proto install` to auto-clean stale plugins after a successful installation.

#### ⚙️ Internal

- Added `PROTO_WASM_LOG` environment variable to toggle the logging of messages from Extism and WASM plugins. Useful for debugging.

## 0.18.5

#### ⚙️ Internal

- Added `PROTO_DEBUG_COMMAND` to include all output when debugging command execution.
- Added more logs to bubble up important information.

## 0.18.4

#### 🐞 Fixes

- Attempts to fix "Failed to parse JSON" errors in relation to the manifest or cached versions.

## 0.18.3

#### 🐞 Fixes

- Another attempt at fixing WASM memory issues.
- Fixed an issue where binaries sometimes could not be located for "installed" tools.

## 0.18.2

#### 🐞 Fixes

- Hopefully fixed an isse where WASM memory was running out of bounds.
- Fixed an issue where failed installs/uninstalls would exit with a zero exit code.

#### ⚙️ Internal

- Fixed an issue where install/uninstall events weren't always firing.

## 0.18.1

#### 🐞 Fixes

- Update our rustls dependency to use OS native certificates.

## 0.18.0

#### 🚀 Updates

- Added a `proto tools` command for listing all installed tools and their versions.
- Added an `http` setting to `~/.proto/config.toml` to control proxies and certificates when making http/https requests, primarily for downloading tools.
  - New `allow-invalid-certs` setting for allowing invalid certificates (be careful).
  - New `proxies` setting for customizing internal proxy URLs.
  - New `root-cert` setting for providing a root certificate (great for corporate environments).

#### 🐞 Fixes

- Fixed `load_git_tags` by automatically filtering tags that end with `^{}` (dereferenced tags).

## 0.17.1

#### 🚀 Updates

- Updated `proto install --pin` to also pin even if the tool has already been installed.
- Updated Windows to use `pwsh` when available.

#### 🐞 Fixes

- Fixed an issue where `proto install` and `proto list-remote` would read from the cache and be unaware of newly released versions upstream.

## 0.17.0

#### 💥 Breaking

- WASM API
  - Updated `exec_command!` to no longer throw on non-zero exit codes. You'll now need to handle failure states manually.

#### 🚀 Updates

- Added Python language support via the `python` identifier.
- Added colors to command line `--help` menus.
- Added canary support to all applicable tools.
  - New `--canary` flag for `proto install`.
  - Canary release will always be the latest, and can be re-installed.
- Updated the following locations to support partial versions and aliases:
  - Tool versions in `.prototools`.
  - Pinning a default version with `proto install --pin`.
  - Setting global version with `proto global`.
  - Setting local version with `proto local`.
- TOML API
  - Added `install.download_url_canary` and `install.checksum_url_canary` settings.
- WASM API
  - Added `command_exists`, `is_musl`, and `get_target_triple` helper functions.
  - Added `skip_install` field to `NativeInstallOutput`.
  - Added `skip_uninstall` field to `NativeUninstallOutput`.

#### ⚙️ Internal

- Now supports `.zst` (or `.zstd`) archive formats.
- Improved version, alias, and requirement handling.

## 0.16.1

#### 🐞 Fixes

- Fixed an issue where `proto clean --purge` would not delete shims.

## 0.16.0

#### 💥 Breaking

- WASM API
  - Requires `extism` >= v0.5.
  - Requires `extism-pdk` >= v0.3.4.

#### 🚀 Updates

- We now include the current proto version in logs.
- Added a `proto add-plugin` command for adding a plugin to a config file.
- Added a `proto remove-plugin` command for removing a plugin from a config file.
- Updated `proto clean` with `--purge` to completely remove a tool from proto.
- Updated `proto clean` with `--purge-plugins` to remove all installed plugins.
- Updated `proto clean` to also remove stale/unused plugins.

#### 🐞 Fixes

- Fixed some commands where their shorthand alias was not being registered correctly.

#### ⚙️ Internal

- Added folder locking during tool installation to avoid colliding processes.
- Renamed `PROTO_ROOT` to `PROTO_HOME`, but we'll support `PROTO_ROOT` for the time being.

## 0.15.1

#### ⚙️ Internal

- Improved file locking logic and scenarios.
- Improved logging to pinpoint slow operations.
- Updated Rust to v1.72.

## 0.15.0

#### 💥 Breaking

- WASM API
  - Removed `env` from all inputs. Use `get_proto_environment` function or `context` input instead.

#### 🚀 Updates

- Added a `proto uninstall-global` command for uninstalling a global dependency from a tool.
- Updated the `proto plugins` command to include the plugin's version when applicable.
- TOML API
  - Added `globals.uninstall-args` to schema, allowing globals to be uninstalled.
- WASM API
  - Added `install_global`, `uninstall_global`, `native_uninstall`, and `sync_shell_profile` plugin functions.
  - Added `pre_install`, `post_install`, `pre_run`, and `post_run` plugin hooks.
  - Added `plugin_version` field to `ToolMetadataOutput`.
  - Added a `VirtualPath` enum for working with virtual and real paths. All `PathBuf` inputs have been updated to this new type.
  - Added a `context` field to some inputs, that includes the plugin ID, tool directory, and current version.
  - Added a `get_tool_id` function for accessing the current plugin ID.
  - Added a `get_proto_environment` function for accessing information about the host and proto environment.

#### 🐞 Fixes

- Fixed an issue where some error messages would be obfuscated.

#### ⚙️ Internal

- The `proto_cli` crate can no longer be used as a library, use `proto_core` instead.

## 0.14.2

#### 🐞 Fixes

- Added file locking around the remote versions cache to avoid fs race conditions.

## 0.14.1

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
