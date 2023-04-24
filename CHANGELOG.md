# Changelog

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
