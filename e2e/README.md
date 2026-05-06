# proto E2E

Real-shell integration tests for the `proto` CLI. Complements the Rust integration tests in [crates/cli/tests](../crates/cli/tests) by exercising what `starbase_sandbox` can't: real shell argv quoting, real `PATH` shim resolution, real `proto activate` shell hooks, real network downloads, and a real shared `PROTO_HOME` populated with all built-in tools.

The whole suite is Bash. The same scripts run on Linux, macOS, and Windows (Git Bash on `windows-latest`). Tests run in numbered order against a shared `PROTO_HOME` and record PASS / FAIL / SKIP with per-test logs.

## Running

```bash
# Build proto and run the full suite
just test-e2e

# Or step-by-step
cargo build --release --bin proto --bin proto-shim
cd plugins && cargo build --workspace --target wasm32-wasip1 --release
cd ..
bash ./e2e/run.sh

# Filter by name (substring match)
bash ./e2e/run.sh install
bash ./e2e/run.sh node
```

Per-test stdout/stderr is captured under `e2e/.logs/<name>.log`. The shared `PROTO_HOME` lives at `e2e/.proto-home` and is wiped at the start of each run.

Override the binary location with `PROTO_BIN_DIR=/some/dir bash ./e2e/run.sh` if you want to test a binary other than `target/release/proto`.

## Layout

| Path                           | Purpose                                                                                                  |
| ------------------------------ | -------------------------------------------------------------------------------------------------------- |
| [run.sh](run.sh)               | Harness. Walks `tests/*.sh` in glob order, parses directives, runs each as a subprocess, prints summary. |
| [lib/env.sh](lib/env.sh)       | Cross-platform env: `PROTO_HOME`, `PATH`, MSYS settings, locale. Sourced by harness and every test.      |
| [lib/assert.sh](lib/assert.sh) | `assert_eq`, `assert_contains`, `assert_exit`, `retry`, etc.                                             |
| [fixtures/](fixtures)          | `.prototools` files copied into temp work dirs by tests.                                                 |
| [tests/](tests)                | Numbered test scripts — see below.                                                                       |

## Test ordering

Tests run alphabetically. The numeric prefix encodes phase, not strict ordering inside a phase:

| Range   | Phase                                                                                |
| ------- | ------------------------------------------------------------------------------------ |
| `00–09` | Smoke (help, version listing)                                                        |
| `10–19` | Standalone tool installs                                                             |
| `20–29` | Dependent tool installs (npm/pnpm/yarn → node, poetry → python)                      |
| `30–39` | Backends (asdf, Unix-only)                                                           |
| `40–49` | Cross-cutting checks (status, bin, run, shim, prototools, exec, outdated, pin/alias) |
| `50-59` | Activation checks (activate, deactivate, etc)                                        |
| `90–99` | Teardown (uninstall, clean)                                                          |

State accumulates across tests — earlier installs are visible to later tests. Don't reorder without considering dependencies.

## Test directives

Three optional comment directives are parsed by `run.sh`:

```bash
#!/usr/bin/env bash
# requires: 10-install-node 14-install-python    # space-separated test names (no .sh)
# os: linux,macos                                # comma-separated allow-list
# group: tools                            # parallel batch label
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"
```

- **`# requires:`** — if any named test FAILED or was SKIPPED, this test is also SKIPPED. Used for tool dependencies (`npm` requires `node`).
- **`# os:`** — allow-list of OSes. Default = all 3. Used for `30-asdf-backend` (`linux,macos`) since the asdf backend doesn't run on Windows.
- **`# group:`** — consecutive tests sharing this value run as background jobs in parallel. The harness waits for the whole group to finish before moving on to the next test. Used to fan out the install phase: `10–18` share `install-base`, `20–23` share `install-deps`. Members of the same group must be independent of each other — `# requires:` should only point to tests in earlier groups or earlier sequential tests, not peers in the same group (peer status isn't recorded until the batch completes).

## Adding a test

1. Create `tests/NN-name.sh` with a numeric prefix that places it in the right phase.
2. Mark it executable: `chmod +x tests/NN-name.sh`.
3. Standard preamble:
   ```bash
   #!/usr/bin/env bash
   # requires: ...    (if it depends on prior tests)
   # os: ...          (if it should skip on some OSes)
   set -euo pipefail
   source "$(dirname "$0")/../lib/env.sh"
   source "$(dirname "$0")/../lib/assert.sh"
   ```
4. Use `retry 3 proto install ...` for any command that hits the network. Don't retry assertions.
5. If you need a clean cwd with a fixture, `mktemp -d`, copy the fixture, `cd` into it, and `trap 'rm -rf "$work"' EXIT`. The harness already gives you a neutral cwd outside the repo (so the repo's own `.prototools` doesn't accidentally activate).
6. Assertion helpers must `return 1` — `set -e` does not propagate inside `if`/`&&`/`||`, so don't rely on it inside conditionals.

## Common pitfalls

- **Don't compare paths with `assert_eq` on macOS.** `mktemp -d` returns `/var/folders/...` symlinked to `/private/var/...`. Use `assert_contains` or `realpath -P` first.
- **Don't pass POSIX-shaped argv on Windows without thinking.** `lib/env.sh` sets `MSYS_NO_PATHCONV=1` and `MSYS2_ARG_CONV_EXCL='*'` to disable Git Bash's path translation, but be aware: any argument that looks like `/something` would otherwise be rewritten to `C:\msys64\something` before reaching `proto.exe`.
- **CRLF will break sourced libs on Windows.** The repo's `.gitattributes` enforces LF for `e2e/**` and `*.sh`. If you see `command not found: $'\r'`, that's CRLF — re-clone with `core.autocrlf=false` or `git config --global core.autocrlf input`.
