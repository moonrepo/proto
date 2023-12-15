# Changelog

## Plugin changelogs

- [Bun](https://github.com/moonrepo/bun-plugin/blob/master/CHANGELOG.md)
- [Deno](https://github.com/moonrepo/deno-plugin/blob/master/CHANGELOG.md)
- [Go](https://github.com/moonrepo/go-plugin/blob/master/CHANGELOG.md)
- [Node](https://github.com/moonrepo/node-plugin/blob/master/CHANGELOG.md)
- [Python](https://github.com/moonrepo/python-plugin/blob/master/CHANGELOG.md)
- [Rust](https://github.com/moonrepo/rust-plugin/blob/master/CHANGELOG.md)
- [TOML schema](https://github.com/moonrepo/schema-plugin/blob/master/CHANGELOG.md)

## 0.25.3

#### 🚀 Updates

- Added `--include-global` to `proto outdated` to include versions from `~/.proto/.prototools`.
- Added `--only-local` to `proto outdated` to only checks versions from `.prototools` in current directory.
- Improved the messaging of `proto outdated`.

#### 🐞 Fixes

- Fixed `proto outdated` checking global versions in `~/.proto/.prototools` by default.

## 0.25.2

#### ⚙️ Internal

- Updated dependencies.

## 0.25.1

#### 🐞 Fixes

- Fixed `proto debug config` printing an invalid config structure.
- Fixed `proto install` displaying the incorrect version labels.
- Fixed `proto install` not always pinning a version on 1st install.

## 0.25.0

#### 🚀 Updates

- Added Linux arm64 gnu and musl support (`aarch64-unknown-linux-gnu` and `aarch64-unknown-linux-musl`).
- Added a `proto debug config` command, to debug all loaded configs and the final merged config.
- Added a `PROTO_BYPASS_VERSION_CHECK` environment variable, to bypass loading and checking of versions. Useful when internet is unreliable.

## 0.24.2

#### 🚀 Updates

- Deferred loading of the HTTP client until it's needed. This should improve execution times.

#### 🐞 Fixes

- Fixed an issue where `proto use` would install tools from `~/.proto/.prototools`.
- Fixed an issue where our directory locking would fail on Windows when the inventory path was overwritten.
- Fixed stable being considered a latest alias.

#### ⚙️ Internal

- Updated dependencies.

## 0.24.1

#### 🚀 Updates

- Added an `--aliases` flag to `proto list` and `proto list-remote`.
- Updated `proto tool list` to include remote aliases provided by the tool.
- Updated `proto tool info` to include local configuration and installed versions.

#### 🧩 Plugins

- Updated `node_plugin` and `node_depman_plugin` to v0.6.1.

## 0.24.0

#### 💥 Breaking

> To ease the migration process, we've added a new migrate command. Simply run `proto migrate v0.24` after upgrading proto!

- Standardized configuration files.
  - Merged `~/.proto/config.toml` functionality into `.prototools` under a new `[settings]` table. This means settings like `auto-clean` can be defined anywhere now.
  - Removed `~/.proto/config.toml`. Use `~/.proto/.prototools` instead, which is now the new global config (via `--global` arg).
  - Moved `node-intercept-globals` setting to `tools.node.intercept-globals`.
- Reworked user configured aliases and default/global version.
  - Moved values to `.prototools` (user managed) from `~/.proto/tools/<name>/manifest.json` (internally managed).
  - Aliases are now stored in the `[tools.<name>]`, while the default version is at the root.
    ```toml
    node = "20.10.0"
    [tools.node.aliases]
    work = "^18"
    ```
- Updated `proto alias` and `proto unalias` to longer write to the global config by default. Now requires a `--global` flag.
  - This change was made to align with `proto pin`, `proto tool add`, and `proto tool remove`.

#### 🚀 Updates

- Added a `proto migrate v0.24` command for migrating configs. We'll also log a warning if we detect the old configuration.
  - For some scenarios, we'll attempt to auto-migrate under the hood when applicable.
- Added support for defining configuration that can be passed to WASM plugins.
  - Can be added to `.prototools` under a `[tools.<name>]` table.
  - Moved Node.js specific settings into this new format.
    ```toml
    [tools.node]
    bundled-npm = false
    intercept-globals = false
    ```
- Updated non-latest plugins to be cached for 30 days, instead of forever.
- Updated cleaning to also remove old proto versions from `~/.proto/tools/proto`.
- WASM API
  - Added a `get_tool_config` function. Can be typed with a serde compatible struct.
  - Deprecated the `get_proto_user_config` function.

#### 🐞 Fixes

- Fixed an issue where resolving canary versions wouldn't work correctly.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.6.
- Updated `deno_plugin` to v0.6.
- Updated `go_plugin` to v0.6.
- Updated `node_plugin` and `node_depman_plugin` to v0.6.
- Updated `python_plugin` to v0.4.
- Updated `rust_plugin` to v0.5.
- Updated `schema_plugin` (TOML) to v0.6.

## 0.23.8

#### 🚀 Updates

- Added a `PROTO_SHELL_PROFILE` environment variable, to control which shell profile to modify (when applicable).
- Added a `PROTO_INSTALL_DIR` environment variable, to control where the `proto` binary is located. Works for both installing and upgrading.

#### 🐞 Fixes

- Fixed `proto upgrade` not working on Linux musl.

## 0.23.7

#### 🐞 Fixes

- Actually fixed `proto use` this time.

## 0.23.6

#### 🚀 Updates

- Enabled wasmtime caching, which should improve performance of WASM plugins by 10-20%.

#### 🐞 Fixes

- Fixed an issue where `proto use` (or parallel processes) would run into file system
  collisions when attempting to download and install multiple TOML schema based tools.

#### ⚙️ Internal

- Updated dependencies.
- Updated parent execution to prefer `proto run <tool>` over `<tool>.exe` on Windows.

## 0.23.5

#### 🚀 Updates

- Updated tools that require execution through a parent tool, to avoid using shims.
- Updated `.cmd` shims to no longer prompt with "Terminate batch job?".

#### 🐞 Fixes

- Fixed executable extension checks on Windows.
- Fixed `.cmd` and `.ps1` shims not exiting correctly.

#### ⚙️ Internal

- Updated dependencies.

## 0.23.4

#### 🐞 Fixes

- Fixed `proto list-global` not resolving a version.

#### 🧩 Plugins

- Updated `node_plugin` and `node_depman_plugin` to v0.5.3.
  - Updated globals install to use a `--prefix` arg instead of `PREFIX` env var.
  - Fixed an incorrect globals directory on Windows.
- Updated `python_plugin` to v0.3.0 (from v0.2.0).
  - Removed `--user` from global package installation via `proto install-global`. Packages are now installed into the tool directory for the current Python version.

## 0.23.3

#### 🚀 Updates

- Updated `proto install-global` and `proto uninstall-global` to detect/resolve a version first, as some package managers require it.
- Updated Windows to _not_ use symlinks for binaries, and instead copy the `.exe` file. This is required to solve "A required privilege is not held by the client" errors, because symlinks require admin privileges.

#### 🐞 Fixes

- Fixed an issue where `proto list-global` would panic when canonicalizing paths.
- Fixed multi-version ranges (`||`) not resolving locally installed versions correctly.

## 0.23.2

#### 🐞 Fixes

- Fixed an issue where checksum verification would fail if the `.sha256` file prefixed the file name with `*`.
- Fixed an issue where installing a global would fail to find a proto shim on Windows.

## 0.23.1

#### 🐞 Fixes

- Fixed an issue where broken symlinks would fail to be removed. This would result in subsequent "File exists (os error 17)" errors.

#### ⚙️ Internal

- Updated Rust to v1.74.
- Updated dependencies.
- Updated logs to now include nanoseconds.

## 0.23.0

#### 💥 Breaking

- Deprecated and moved tool/plugin commands to `proto tool` subcommand.
  - Moved `proto add-plugin` to `proto tool add`.
  - Moved `proto remove-plugin` to `proto tool remove`.
  - Moved `proto plugins` to `proto tool list-plugins`.
  - Moved `proto tools` to `proto tool list`.

#### 🚀 Updates

- Added a `proto tool info` command for viewing information about a tool and its plugin.
- Added a `detect-strategy` setting to `~/.proto/config.toml` to configure which strategy to use when detecting a version. Accepts:
  - `first-available` (default) - Will use the first available version that is found. Either from `.prototools` or a tool specific file (`.nvmrc`, etc).
  - `prefer-prototools` - Prefer a `.prototools` version, even if found in a parent directory. If none found, falls back to tool specific file.
- Added support to plugins to ignore certain paths when detecting a version.
- Updated Windows to create 3 shim files for each tool: `.cmd` (cmd.exe), `.ps1` (powershell), and no extension (bash).
- WASM API
  - Added `DetectVersionOutput.ignore` field.

#### 🧩 Plugins

- Updated `node_plugin` and `node_depman_plugin` to v0.5.2.
- **Node**
  - Will now ignore detecting versions from `node_modules` paths.
  - Fixed Yarn v1.22.x archive not unpacking correctly.

## 0.22.2

#### 🐞 Fixes

- Fixed an issue where version detection would read files found in `node_modules` (which you usually don't want).

## 0.22.1

#### 🐞 Fixes

- Fixed an issue where `proto clean` or `proto use` (with auto-clean) would crash.

## 0.22.0

#### 🚀 Updates

- Refactored and standardized how executables (bins and shims) are managed.
  - Binaries (`~/.proto/bin`) and shims (`~/.proto/shims`) now share the same internal data structures.
  - For the most part, is a 1:1 relation. There will be a shim for every binary, and vice versa.
  - Reduced the amount of WASM calls to locate executables to 1 call.
  - Removed the concept of local shims (was a hidden implementation detail).
- Reworked the `proto bin` command.
  - By default returns an absolute path to the real executable (`~/.proto/tools/<tool>/<version>/bin`).
  - Pass `--bin` to return the `~/.proto/bin` path.
  - Pass `--shim` to return the `~/.proto/shims` path.
- Updated `proto clean --purge` and `proto uninstall` to accurately delete all executables.
- Updated `proto uninstall` to support removing the tool entirely (simply omit the version).
- Updated internet connection checks to only check during critical workflows.
  - Will no longer happen if we have a fully-qualified version (primarily for `proto run`).
  - Will still happen for partial versions, as we need to resolve to a fully-qualified.
  - Will always happen for install, upgrade, and other flows that must download files.
- TOML API
  - Added `install.no_bin` and `install.no_shim` fields.
- WASM API
  - Added `locate_executables` function.
  - Added `LocateExecutablesInput`, `LocateExecutablesOutput`, `ExecutableConfig` structs.
  - Deprecated `locate_bins` and `create_shims` functions.
  - Deprecated `LocateBinsInput`, `LocateBinsOutput`, `CreateShimsInput`, `CreateShimsOutput`, `ShimConfig` structs.

#### 🐞 Fixes

- Fixed an issue where config files in the user home directory were not loaded.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.5.
- Updated `deno_plugin` to v0.5.
- Updated `go_plugin` to v0.5.
- Updated `node_plugin` and `node_depman_plugin` to v0.5.
- Updated `python_plugin` to v0.2.
- Updated `rust_plugin` to v0.4.
- Updated `schema_plugin` (TOML) to v0.5.
- **Node**
  - Updated the `npm` tool to create the `npx` shim instead of the `node` tool.
  - Updated symlinked binaries to use the shell scripts instead of the source `.js` files (when applicable).

#### ⚙️ Internal

- Plugin versions are now pinned and tied to proto releases to avoid unintended drift and API changes.

## 0.21.1

#### 🐞 Fixes

- Fixed an issue where uninstalling the "default version" doesn't delete the symlinked binary.
- Fixed an issue where the versions cache was not being read.
- Fixed an issue where installing a tool would write to the wrong temporary directory.

## 0.21.0

#### 💥 Breaking

- WASM API
  - Removed `input.context` from `LoadVersionsInput` and `ResolveVersionInput`.

#### 🚀 Updates

- Added Linux x64 musl support (`x86_64-unknown-linux-musl`).
- Improved file and directory locking. Will now work correctly across processes and signals, especially for those killed/dropped.
- Updated WASM functions to use explicit Rust enum types for versions to properly handle all variations (version, alias, requirement, range).
- WASM API
  - Uses `VersionSpec` enum:
    - `ResolveVersionOutput.version`
    - `ToolContext.version`
  - Uses `UnresolvedVersionSpec` enum:
    - `LoadVersionsInput.initial`
    - `ParseVersionFileOutput.version`
    - `ResolveVersionInput.initial`
    - `ResolveVersionOutput.candidate`
    - `SyncManifestOutput.default_version`
    - `ToolMetadataOutput.default_version`

## 0.20.4

#### 🐞 Fixes

- Fixed an issue where auto-install would keep re-installing a tool.
- Fixed more WASM memory issues.

## 0.20.3

#### 🚀 Updates

- Added a `PROTO_OFFLINE_TIMEOUT` environment variable to control the timeout for offline checks (in milliseconds).
- Added a `PROTO_OFFLINE_HOSTS` environment variable to customize additional hosts/IPs to check for offline status.
- WASM API
  - Updated `host_log!` to support writing to stdout/stderr.

#### 🐞 Fixes

- Fixed `proto migrate` failing on Windows.

#### ⚙️ Internal

- Added more logging to WASM functions.

## 0.20.2

#### 🚀 Updates

- Improved offline checks and behaviors.

#### 🐞 Fixes

- Fixed a WASM memory issue that would error with "extism_call failed".
- Fixed an issue where virtual paths would be mis-prefixed.

#### ⚙️ Internal

- Renamed `/home` virtual path to `/userhome` to avoid conflicts.
- Updated dependencies.

## 0.20.1

#### 🚀 Updates

- Updated `proto use` to load plugins in parallel.

#### 🐞 Fixes

- Fixed an issue where `proto use` would not bubble up errors for tools that fail to install.

#### ⚙️ Internal

- Increased the timeout for WASM function calls from 30s to 90s.
- Improved and clarified some error messages.

## 0.20.0

#### 💥 Breaking

> To ease the migration process, we've added a new migrate command. Simply run `proto migrate v0.20` after upgrading proto!

- The generated shims have moved to `~/.proto/shims` from `~/.proto/bin`. You'll need to manually update `PATH` in your shell profile if you'd like to continue using the "runtime version detection" functionality.

  ```diff
  export PROTO_HOME="$HOME/.proto"
  -export PATH="$PROTO_HOME/bin:$PATH"
  +export PATH="$PROTO_HOME/shims:$PROTO_HOME/bin:$PATH"
  ```

  Furthermore, we suggest deleting all files in `~/.proto/bin` except for `proto(.exe)`.

- WASM API
  - Removed `env_vars` from `ToolMetadataOutput` and `ToolContext`. Use `host_env!` macro instead.

#### 🚀 Updates

- Reworked the `~/.proto/bin` directory to now contain symlinks to the original tool executables. This is a non-shim based alternative that can be used stand-alone or in unison with our shims.
  - The globally pinned version is the version that's symlinked. This can be updated with `proto install --pin`.
  - This approach _does not_ detect a version at runtime.
- Added a `proto migrate` command for easily applying changes between breaking releases.
- Added support for minisign checksum files. Can now verify `.minisig` signatures for downloaded tools.
- Updated `proto use` to install tools in parallel.
- Updated `proto plugins` and `proto tools` to load plugins in parallel.
- Updated `proto run` to error when the tool attempts to self-upgrade outside of proto.
- TOML API
  - Added a `metadata` setting.
  - Added a `install.checksum-public-key` setting.
- WASM API
  - Added a `self_upgrade_commands` field to `ToolMetadataOutput`.
  - Added a `checksum_public_key` field to `DownloadPrebuiltOutput`.
  - Removed `checksum` from `VerifyChecksumInput`.

#### ⚙️ Internal

- Minor performance improvements to runtime version detection.
- Improved error handling and messages.

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
