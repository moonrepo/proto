# Changelog

## Plugin changelogs

- [Bun](https://github.com/moonrepo/tools/blob/master/tools/bun/CHANGELOG.md)
- [Deno](https://github.com/moonrepo/tools/blob/master/tools/deno/CHANGELOG.md)
- [Go](https://github.com/moonrepo/tools/blob/master/tools/go/CHANGELOG.md)
- [Node](https://github.com/moonrepo/tools/blob/master/tools/node/CHANGELOG.md)
- [Python](https://github.com/moonrepo/tools/blob/master/tools/python/CHANGELOG.md)
- [Rust](https://github.com/moonrepo/tools/blob/master/tools/rust/CHANGELOG.md)
- [TOML schema](https://github.com/moonrepo/tools/blob/master/tools/internal-schema/CHANGELOG.md)

## Unreleased

- Removed `--include-global` and `--only-local` flags from all applicable commands. Use the new `--config-mode` instead.

#### 🚀 Updates

- Added a new `--config-mode` global option that controls how configuration are loaded.
  - Supports the following values:
    - `global` - Only load `~/.proto/.prototools`.
    - `local` - Only load `./.prototools` in the current directory.
    - `upwards` - Load `.prototools` while traversing upwards, but do not load `~/.proto/.prototools`.
    - `upwards-global` - Load `.prototools` while traversing upwards, and do load `~/.proto/.prototools`.
  - When not provided, the default mode is dependent on the command being ran.
    - For `activate`, `install`, `outdated`, `status` -> `upwards`
    - Everything else -> `upwards-global`

## 0.39.5

#### 💥 Breaking

- We've discovered a bug with the new "pin proto version" feature that will require non-trivial amount of work to resolve correctly. However, to resolve the bug at this time, the pinning functionality will now only work if you run `proto activate` in your shell.

#### 🚀 Updates

- Improved performance slightly for `proto activate`.
- Improved the logic around cleaning the `~/.proto/tools/proto` directory.
- Updated the `auto-clean` setting to automatically run in more contexts.

## 0.39.4

#### 🐞 Fixes

- Fixed an issue where `proto activate --include-global` would not pass arguments down to its child processes.

## 0.39.3

#### 🚀 Updates

- Added `--check` and `--json` options to `proto upgrade`.
- Added an explicit version argument to upgrade/downgrade to for `proto upgrade`.

#### 🐞 Fixes

- Fixed an issue where `proto upgrade` may error with access denied when renaming binaries.

#### 🧩 Plugins

- Updated `node_tool` to v0.11.8.
  - Fixed macOS nightly detection.

## 0.39.2

#### 🚀 Updates

- Updated `proto diagnose` to check the current proto version.

#### 🐞 Fixes

- Disabled the version check requests for `proto activate`.

#### 🧩 Plugins

- Updated `deno_tool` to v0.11.4.
  - Updated canary to find an applicable release based on the current os/arch combination.
- Updated `node_tool` to v0.11.7.
  - Updated canary to find an applicable release based on the current os/arch combination.

## 0.39.1

#### 🚀 Updates

- Updated `proto activate` to set `PROTO_HOME` if it is not defined in the current shell.

#### 🐞 Fixes

- Fixed a performance regression on `proto activate`. Should exit immediately now.

## 0.39.0

#### 💥 Breaking

- Updated `proto activate` to not include tools without a configured version and to not include global tools found in `~/.proto/.prototools` by default.
  - If you would like to include global tools, pass `--include-global`.
  - Do be aware that having a lot of global tools will cause considerable performance loss when activation is triggered.

#### 🚀 Updates

- Added a new setting to `.prototools`, `settings.builtin-plugins`, that can be used to disable all built-in plugins, or only allow a few select plugins.
  - Supports a boolean or list of plugin names.
  - All are enabled by default for backwards compatibility.
- Added `PROTO_NO_MODIFY_PROFILE` and `PROTO_NO_MODIFY_PATH` environment variables to `proto setup` (for automated workflows).
- Updated `proto status` to display and include versions from ecosystem files (`.nvmrc`, etc).
- Updated `github://` plugin locators to support monorepos. Append the project name (that tags are prefixed with) to the path: `github://moonrepo/tools/node_tool`
- Merged `proto use` and `proto install` commands. If no arguments are provided to `proto install`, it will install all configured tools.
- You can now pin a version of proto itself within `.prototools`, and proto shims will attempt to run proto using that configured version, instead of the global version.
  ```toml
  proto = "0.38.0"
  ```

#### 🧩 Plugins

- Updated `go_tool` to v0.12.
  - Changed the `gobin` setting to `false` by default.
- Updated `node_depman_tool` to v0.12.
  - Added a `dist-url` config setting, allowing the download host to be customized.

#### ⚙️ Internal

- Updated Rust to v1.80.

## 0.38.4

#### 🐞 Fixes

- Attempted fix for some certificate issues when making requests.

## 0.38.3

#### 🧩 Plugins

- Updated `go_plugin` to v0.11.4.
  - Fixed `go.mod`/`go.work` version detection/parsing on Windows.
- Updated `node_depman_plugin` to v0.11.6.
  - Fixed the shared globals directory not resolving correctly.

## 0.38.2

#### 🚀 Updates

- Improved our logic around "update shell profile if not already setup".

#### 🐞 Fixes

- Fixed powershell syntax when joining paths and environment variables.

## 0.38.1

#### 🚀 Updates

- Support `.x` when parsing versions. Will be treated as `*`.

#### 🐞 Fixes

- Fixed and removed some "unreachable" branches when parsing versions.

## 0.38.0

#### 💥 Breaking

- While not a direct breaking change, we've added escaping/quoting logic to shell injected values, which was required for the new activation workflow. Please report an issue on GitHub or Discord if the value we injected has incorrect syntax!

#### 🚀 Updates

- Added an experimental command called `proto activate` that can be ran within your shell profile to "activate the proto environment", by setting necessary environment variables and paths when changing directories.
  - Globally installed packages will now be available automatically. This wasn't possible through shims alone.
  - Binaries that come pre-installed with a tool (and are not shims) will also be available automatically.
- Added support for [murex](https://murex.rocks/) shells.
- Added a `--include-global` flag to `proto use`, that will also install globally configured tools.
- WASM API
  - Added `LocateExecutablesOutput.exes_dir` field.

#### 🧩 Plugins

- Updated `node_plugin` and `node_depman_plugin` to v0.11.4.
- Updated `python_plugin` to v0.10.4.
- Updated `rust_plugin` to v0.10.5.
  - Respect `CARGO_HOME` during rustup installation.
- Updated `schema_plugin` (TOML) to v0.14.0.
  - Added `platform.*.exes_dir`.
  - Renamed `platform.*.bin_path` to `exe_path`.

## 0.37.2

#### 🐞 Fixes

- Fixed `proto upgrade` not working correctly when the release is in progress, or not available yet.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.12.2.
- Updated `deno_plugin` to v0.11.2.
- Updated `go_plugin` to v0.11.2.
- Updated `node_plugin` and `node_depman_plugin` to v0.11.3.
- Updated `python_plugin` to v0.10.2.
- Updated `rust_plugin` to v0.10.3.
  - Updated `RUSTUP_HOME` to support relative paths.
- Updated `schema_plugin` (TOML) to v0.13.1.
  - Updated `resolve.aliases` to support ranges, requirements, and aliases.
  - Updated `resolve.version-pattern` and `resolve.git-tag-pattern` to support year/month/day regex group names (for calver support).

## 0.37.1

#### 🧩 Plugins

- Updated `node_plugin` and `node_depman_plugin` to v0.11.2.
  - Fixed yarn "2.4.3" not resolving or downloading correctly.
- Updated `python_plugin` to v0.10.2.
  - Will now create a pip shim that includes the major version, for example, `pip3`.

## 0.37.0

#### 💥 Breaking

- WASM API
  - Changed `SyncManifestOutput` `versions` field to the `VersionSpec` type instead of `Version`.
  - Changed `LoadVersionsOutput` `canary`, `latest`, and `aliases` fields to the `UnresolvedVersionSpec` type instead of `Version`.
  - Changed `LoadVersionsOutput` `versions` fields to the `VersionSpec` type instead of `Version`.
  - Renamed `VersionSpec::Version` to `VersionSpec::Semantic`. The inner `Version` must also be wrapped in a `SemVer` type.

#### 🚀 Updates

- Added experimental support for the [calver](https://calver.org) (calendar versioning) specification. For example: 2024-04, 2024-06-10, etc.
  - There are some caveats to this approach. Please refer to the documentation.
  - This _should_ be backwards compatible with existing WASM plugins and tools, but in the off chance it is not, please pull in the new PDKs and publish a new release, or create an issue.
- Added a new command, `proto diagnose`, that can be used to diagnose any issues with your current proto installation.
  - Currently diagnoses proto itself, but in the future will also diagnose currently installed tools.
- WASM API
  - Added `VersionSpec::Calendar` and `UnresolvedVersionSpec::Calendar` variant types.

#### ⚙️ Internal

- Improved command execution. May see some slight performance gains.
- Updated wasmtime to v21 (from v17).
- Updated Rust to v1.79.

## 0.36.2

#### 🚀 Updates

- Added Nushell support to `proto completions`.

## 0.36.1

#### 🚀 Updates

- Improved logic to detect the `proto-shim` binary when proto is installed in non-standard locations.

## 0.36.0

#### 🚀 Updates

- Added a `proto plugin search` command that can be used to search for community created plugins.
- Added a `proto unpin` command, for removing a pinned version from a `.prototools` file.
- Updated `proto uninstall` to also remove entries from `.prototools` if the version was uninstalled.
- Updated plugin locator strings to use common protocol syntax. The custom `source:` syntax is deprecated.
  - `source:./file.wasm` -> `file://./file.wasm`
  - `source:https://url.com/file.wasm` -> `https://url.com/file.wasm`
  - `github:org/repo` -> `github://org/repo`
- Updated some error messages to include copy for work arounds.

#### 🐞 Fixes

- Fixed invalid `PATH` syntax for Elvish shell.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.12.1.
- Updated `deno_plugin` to v0.11.1.
- Updated `go_plugin` to v0.11.1.
- Updated `node_plugin` and `node_depman_plugin` to v0.11.1.
- Updated `python_plugin` to v0.10.1.
- Updated `rust_plugin` to v0.10.1.
- Updated `schema_plugin` (TOML) to v0.13.0.
  - Added `resolve.aliases` and `resolve.versions` settings, allowing an explicit list of aliases and versions to be defined.

#### ⚙️ Internal

- We now lock the bin/shims directory when creating/removing files.
  - This is an experiment to help avoid race conditions where multiple proto processes are all trying to write to the same location.
  - If this results in too large of a performance hit, we'll remove the locking.
- Reworked how serde defaults are applied for input/output plugin function types.

## 0.35.5

#### 🐞 Fixes

- Fixed version parsing when ranges included a leading `v`, for example `>=v18.0.0`.

## 0.35.4

#### 🐞 Fixes

- Fixed some scenarios where the shims were unnecessarily being created.

## 0.35.3

#### 🐞 Fixes

- Attempted fix for the "inventory directory has been overridden" error (primarily happens with the Rust plugin).

## 0.35.2

#### 🚀 Updates

- Added a `PROTO_DEBUG_SHIM` environment variable, which will output some debug information for the shim executable. This will help uncover issues with the shim itself.

## 0.35.1

#### 🧩 Plugins

- Updated `go_plugin` to v0.11.1.
  - Added `gofmt` as a secondary shim/binary.
  - Updated `go.mod` version parsing to use better ranges.

## 0.35.0

#### 🚀 Updates

- Added experimental support for the following shells: ion, nu, xonsh.
- Added a global `--dump` flag, that will dump a trace profile that can be inspected in `chrome://tracing`.
  - Is not supported for `proto run` or when running a tool shim.
- Updated `proto setup` to prompt the user to select a shell if one could not be detected.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.12.0.
- Updated `deno_plugin` to v0.11.0.
- Updated `go_plugin` to v0.11.0.
- Updated `node_plugin` and `node_depman_plugin` to v0.11.0.
- Updated `python_plugin` to v0.10.0.
- Updated `rust_plugin` to v0.10.0.
- Updated `schema_plugin` (TOML) to v0.12.

#### ⚙️ Internal

- Updated Rust to v1.78.
- Started on "build from source" APIs.

## 0.34.4

#### 🚀 Updates

- Added `.zshenv` as a valid shell profile option.

#### 🐞 Fixes

- Fixed `proto outdated --update` erasing other content in the file.

## 0.34.3

#### 🐞 Fixes

- Fixed some edge cases around version resolving.

## 0.34.2

#### 🐞 Fixes

- Another attempted fix for `proto outdated` latest checks.

#### ⚙️ Internal

- Added a lot of trace logs around version resolving.

## 0.34.1

#### 🐞 Fixes

- Fixed an issue where global versions would overwrite local versions in `proto status` and `proto outdated`.
- Fixed an issue where the "latest" alias would sometimes not be resolved.

#### 🧩 Plugins

- Updated `node_plugin` and `node_depman_plugin` to v0.10.3.
  - Fixed yarn "latest" alias pointing to the v1 latest, instead of v4 (berry) latest.

## 0.34.0

#### 💥 Breaking

- Updated `proto install --pin` to pin to the local `.prototools` by default, instead of the global `~/.proto/.prototools`. This now aligns with the other pinning commands and args.

#### 🚀 Updates

- Added a new option for the `detect-strategy` setting, `only-prototools`, that only reads versions from `.prototools`, and not tool related files.
- Added a new command, `proto status`, that shows the status of active tools.
  - An active tool is one that has been configured in `.prototools`.
  - Includes the resolved version, install path, configured path, and more.
- Updated `proto outdated` with a better developer experience.
  - Displays all tools and available updates as a table of data.
  - Includes newest (matching range) and latest versions available.
  - Includes the config file that the tool + version was loaded from.
  - When `--update` is passed, will now prompt to confirm the update.
  - When updating versions, versions are now written to their original config file, instead of local.
  - Updated the `--latest` flag to use the latest version when updating, instead of newest.
- Updated `proto setup` (which is ran during proto installation) to modify the `PATH` system environment variable on Windows. To disable this functionality, pass `--no-modify-path`.

#### 🧩 Plugins

- Added a `dist-url` setting, allowing the distribution download URL to be customized, for the following plugins:
  - bun, deno, go, node
- Updated `bun_plugin` to v0.11.1.
- Updated `deno_plugin` to v0.10.2.
- Updated `go_plugin` to v0.10.2.
- Updated `node_plugin` and `node_depman_plugin` to v0.10.2.

#### 🐞 Fixes

- Fixed `proto clean` accidentally deleting older proto version shims.

## 0.33.0

> This version failed to build correctly, so had to bump another minor.

## 0.32.2

#### 🧩 Plugins

- Updated `bun_plugin` to v0.11.0.
  - Added Windows support.
  - Will now use the baseline build on x64 Linux when available.

## 0.32.1

#### 🐞 Fixes

- Fixed an issue where the version suffix was being stripped from non-version folders in the tool directory. Primarily affects Rust.

#### ⚙️ Internal

- Updated Rust to v1.77.

## 0.32.0

#### 💥 Breaking

- Removed the `PROTO_INSTALL_DIR` environment variable, use `PROTO_HOME` instead.
- Removed the deprecated `/workspace` virtual path prefix, use `/cwd` instead.
- Rewrote the `proto_pdk_test_utils` crate from the ground up to be easier to use.

#### 🚀 Updates

- Cleaned up command stdout and stderr messaging.
- Updated some commands to exit with a non-zero code when data or requirements are missing.
- Implemented a new store structure/layout system for better reliability.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.10.1.
- Updated `deno_plugin` to v0.10.1.
- Updated `go_plugin` to v0.10.1.
- Updated `node_plugin` and `node_depman_plugin` to v0.10.1.
- Updated `python_plugin` to v0.9.
  - Will now create a secondary executable that includes the major version in the file name, for example, `python3`.
- Updated `rust_plugin` to v0.9.1.
- Updated `schema_plugin` (TOML) to v0.11.
  - Added `install.primary` and `install.secondary` settings, for advanced configuring executables.
  - Updated `platform.*.bin-path` to support interpolation.

#### ⚙️ Internal

- Updated dependencies.

## 0.31.5

#### 🐞 Fixes

- Fixed an issue where incorrect newlines were being written to PowerShell profiles.

## 0.31.4

#### 🐞 Fixes

- Fixed `proto clean` and `proto setup` hanging in CI or Docker waiting for input.

## 0.31.3

#### 🚀 Updates

- Updated plugin IDs to support underscores.

#### 🐞 Fixes

- Fixed `.gz` wrapped files not being executable after being unpacked.

## 0.31.2

#### 🐞 Fixes

- Fixed non-tar associated `.gz` archives not being unpacked correctly.
- Fixed musl checks failing on Alpine Linux.

#### 🧩 Plugins

- Updated `schema_plugin` (TOML) to v0.10.1.
  - Added an `install.libc` setting, to customize the libc wording used.

## 0.31.1

#### 🐞 Fixes

- Fixed the globals directory not always being populated correctly. This is required for `shared-globals-dir`.

## 0.31.0

In preparation for an official v1 release, improved stability, and overall developer experience, we're renaming some commands, and removing the "global packages" functionality.

#### 💥 Breaking

- Renamed the `proto tool` commands to `proto plugin`.
- Removed the `proto tool list-plugins` command, and merged its functionality into `proto plugin list`.
- Removed the `proto install-global`, `proto list-global`, and `proto uninstall-global` commands.
- Removed support for the old user config feature (`~/.proto/config.toml`) which was removed in v0.24.
- Removed support for `aliases` and `default_version` in the tool manifest, which was also removed in v0.24.
- Removed the `proto migrate 0.20` and `proto migrate 0.24` commands.
- WASM API
  - Removed `get_tool_id` and `get_proto_environment` helper functions.
  - Removed `install_global` and `uninstall_global` plugin functions.
  - Removed `InstallGlobalInput`, `InstallGlobalOutput`, `UninstallGlobalInput`, `UninstallGlobalOutput` types.

#### 🚀 Updates

- Added a `--resolve` option to `proto pin`, which will resolve the version to a valid value before pinning.
- Added `--aliases` and `--versions` options to `proto plugin list`.
- Added aliases to `proto plugin info`.
- Updated `--pin` option in `proto install` to support "local" and "global" values, allowing the config location to be customized.
  - When `--pin` is passed without a value, will default to "global" for backwards compatibility.
- WASM API
  - Updated the `pre_run` hook to return a result, allowing args/env vars to be injected into the running command.

#### 🐞 Fixes

- Fixed an issue where empty version strings were being parsed, causing failures.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.10.
- Updated `deno_plugin` to v0.10.
- Updated `go_plugin` to v0.10.
- Updated `node_plugin` and `node_depman_plugin` to v0.10.
  - Removed the `intercept-globals` config setting.
  - Added a new `shared-globals-dir` setting, which injects args/env vars into npm/pnpm/yarn commands when they attemp to install global packages.
- Updated `python_plugin` to v0.8.
- Updated `rust_plugin` to v0.9.
- Updated `schema_plugin` (TOML) to v0.10.
  - Removed `globals` and `shims` settings.
  - Added a `platform.*.archs` setting, to limit what architectures are supported for that OS.
  - Added a `packages` setting for managing global packages. Supports `globals_lookup_dirs` and `globals_prefix` sub-settings.

## 0.30.2

#### 🧩 Plugins

- Updated `deno_plugin` to v0.9.1.
  - Added Linux ARM64 support (requires Deno >= v1.41).

#### ⚙️ Internal

- Updated dependencies.

## 0.30.1

#### 🧩 Plugins

- Updated `rust_plugin` to v0.8.1.
  - Uses the full triple target when installing and uninstalling toolchains.
- Updated `schema_plugin` (TOML) to v0.9.1.
  - Updated our regex to support Perl syntax like `\d`.

#### ⚙️ Internal

- Updated Rust to v1.76.

## 0.30.0

#### 💥 Breaking

- WASM API
  - Updated `VirtualPath::real_path` to return an `Option` if conversion fails.

#### 🚀 Updates

- Updated `proto setup` (which is ran during proto installation) to be interactive.
  - Will now prompt you to choose which shell profile to modify, or not at all.
  - Improved the output messages based on the state of the install.
  - Added `--no-profile` and `--yes` arguments to control this.

#### 🐞 Fixes

- Attempted fix for PowerShell profile updates when using Windows 11 and OneDrive folders.

#### 🧩 Plugins

- Updated `node_plugin` and `node_depman_plugin` to v0.9.1.
  - Added version detection support for `volta` in `package.json`.

#### ⚙️ Internal

- Updated dependencies.

## 0.29.1

#### 🐞 Fixes

- Fixed virtual path conversion issues on Windows (mostly affected `rust-plugin`).

## 0.29.0

#### 💥 Breaking

- WASM API
  - Renamed `err!` macro to `plugin_err!`.
  - Renamed `get_tool_id` to `get_plugin_id`.
  - Renamed `get_proto_environment` to `get_host_environment`.
  - Renamed `/workspace` virtual path to `/cwd`.
  - Renamed `ExecCommandInput.env_vars` to `env`.
  - Removed `HostEnvironment.proto_dir` field.
  - Updated `plugin_err!` result to not be wrapped in `Err`.
  - Updated `VirtualPath::join` to return `VirtualPath` instead of `PathBuf`.

#### 🚀 Updates

- Added `env` and `tools.*.env` settings to `.prototools` to support environment variables.
  - Will automatically set variables when a tool is executed.
  - Allows for directory-level and tool-specific variables.
- Added support for environment based config files, like `.prototools.production`.
  - Takes higher precedence than `.prototools`.
  - Can be enabled with the `PROTO_ENV` environment variable.
- Updated `proto tool info` to display aliases and environment variables.
- Updated WASM logs to be shown alongside proto's `--log` output (under the `extism::pdk` namespace).
- WASM API
  - Added color support to error and host log messages through an HTML-like tag syntax.
  - Added `real_path!(buf, ..)` and `virtual_path!(buf, ..)` macro variants for working with `Path` and `PathBuf`.
  - Added a `fetch_url_bytes` function.
  - Improved the implementation of many PDK macros.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.9.
- Updated `deno_plugin` to v0.9.
- Updated `go_plugin` to v0.9.
- Updated `node_plugin` and `node_depman_plugin` to v0.9.
  - Changed the `bundled-npm` and `intercept-globals` settings to be `false` by default (instead of `true`).
- Updated `python_plugin` to v0.7.
- Updated `rust_plugin` to v0.8.
- Updated `schema_plugin` (TOML) to v0.9.
  - Added `globals.bin` setting, for customizing the bin used for installs/uninstalls.

## 0.28.2

#### ⚙️ Internal

- Updates in preparation for the v0.29 release.

## 0.28.1

#### 🚀 Updates

- WASM API
  - Updated the `set_env_var` host function (and `host_env!` macro) to append `PATH` instead of overwriting it, and to also support virtual paths.

#### 🐞 Fixes

- Fixed `proto upgrade` moving the old binary to the wrong location.

#### 🧩 Plugins

- Updated `rust_plugin` to v0.7.1.

## 0.28.0

This release primarily upgrades our WASM runtime. For plugin authors, you'll need to update to the latest PDK and publish a new version. Old plugins are incompatible.

#### 💥 Breaking

- WASM API
  - Refactored the `HostLogInput` enum into a struct (this should be transparent if using the `host_log!` macro).
  - Removed support for old and deprecated APIs: `locate_bins`, `create_shims`.
  - Removed `get_proto_user_config` and `format_bin_name` functions.
  - Updated `get_tool_id` to return a `Result<String>` instead of `String`.
- WASM test utils
  - Removed `WasmTestWrapper::set_environment()` method. Use `create_plugin_with_config` and pass a config object instead.

```rust
// Before
let mut plugin = create_plugin("test-id", sandbox.path());
plugin.set_environment(HostEnvironment {
    arch: HostArch::Arm64,
    os: HostOS::Linux,
    ..Default::default()
});

// After
let plugin = create_plugin_with_config(
  "test-id",
  sandbox.path(),
  HashMap::from_iter([map_config_environment(HostOS::Linux, HostArch::Arm64)]),
);
```

#### 🚀 Updates

- Will now display an upgrade message when the current proto version is out of date.
- Improved error messages when a system command does not exist.
- Improved error messages to include the plugin identifier when applicable.
- Updated our "last used at" logic to avoid race conditions with the tool manifest.
- WASM API
  - Added `from_virtual_path` and `to_virtual_path` host functions.
  - Added `virtual_path!` and `real_path!` macros.
  - Added `ExecCommandInput.working_dir` field.

#### 🐞 Fixes

- Fixed an issue where command executions from the context of a plugin weren't taking virtual paths into account.

#### 🧩 Plugins

- Updated `bun_plugin` to v0.8.
- Updated `deno_plugin` to v0.8.
- Updated `go_plugin` to v0.8.
- Updated `node_plugin` and `node_depman_plugin` to v0.8.
- Updated `python_plugin` to v0.6.
- Updated `rust_plugin` to v0.7.
- Updated `schema_plugin` (TOML) to v0.8.

#### ⚙️ Internal

- Updated Extism (plugin runtime) to v1 (from v0.5).

## 0.27.1

#### 🐞 Fixes

- Fixed broken `proto regen` output.

## 0.27.0

#### 🚀 Updates

- Added a `proto regen` command, that can be used to regenerate shims, and optionally relink bins.
- Updated `proto setup` and the installation script to support PowerShell profiles.
  - Will no longer use `setx` commands on Windows.

#### 🧩 Plugins

- Updated `schema_plugin` (TOML) to v0.7.1.
  - Now uses named regex captures for better version parsing.

#### ⚙️ Internal

- Updated Rust to v1.75.

## 0.26.5

#### 🐞 Fixes

- Fixed an issue where shims wouldn't work when executing with a different file string case.

## 0.26.4

#### 🚀 Updates

- Added more lookup directories when locating the `proto-shim` file.
- Updated the CLI to set the `PROTO_VERSION` environment variable.

## 0.26.3

#### 🐞 Fixes

- Avoid force creating shims in CI when not necessary.

## 0.26.2

#### 🐞 Fixes

- Fixed the `bun_plugin` being set to an incorrect version.
- Temporarily fixed "Access is denied" errors on Windows when creating shims.
- More improvements for the Elvish shell.

## 0.26.1

#### 🚀 Updates

- Added a `proto debug env` command, for debugging basic env/store information.
- Updated version resolve errors to include the tool that failed.
- Updated missing install errors to include the file that a version was detected from.

#### 🐞 Fixes

- Fixed `proto setup` injecting incorrect shell configuration for Elvish.

#### ⚙️ Internal

- Temporarily clean old binaries that are no longer supported.

## 0.26.0

#### 💥 Breaking

- Removed old and deprecated CLI commands.
- WASM API
  - Removed the `post_run` hook.

#### 🚀 Updates

- Implemented a new shim strategy for both Unix and Windows.
  - No longer creates Bash scripts on Unix, or PowerShell scripts on Windows.
  - Instead creates a new Rust based executable that is truly native.
  - Handles stdin, pipes, and redirects efficiently and correctly.
  - Better compatibility and portability.
- WASM API
  - Added a `ToolContext.proto_version` field.
  - Added a `ExecutableConfig.shim_env_vars` field.
  - Updated `ExecutableConfig.shim_before_args` and `ExecutableConfig.shim_after_args` to support a list of strings.

#### 🐞 Fixes

- Fixed an issue where binaries were being symlinked with broken versions in their file name (most commonly for Python).

#### 🧩 Plugins

- Updated `bun_plugin` to v0.7.
  - Will now symlink a `bunx` binary to `~/.proto/bin`.
- Updated `deno_plugin` to v0.7.
- Updated `go_plugin` to v0.7.
- Updated `node_plugin` and `node_depman_plugin` to v0.7.
  - Will no longer symlink binaries (`~/.proto/bin`) for all package managers.
  - You'll most likely need to delete any old bins manually.
- Updated `python_plugin` to v0.5.
- Updated `rust_plugin` to v0.6.
- Updated `schema_plugin` (TOML) to v0.7.

#### ⚙️ Internal

- Added basic telemetry to track tool install/uninstall metrics.

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
