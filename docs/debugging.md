# Debugging proto

Debugging proto is straight-forward, simply place `println!` or `dbg!` statements throughout the codebase, and build a debug target with `cargo build --workspace`.

Now instead of executing the global `proto` binary, execute the local `target/debug/proto`.

## Environment variables

A few environment variables are available to help with debugging:

- `PROTO_LOG=trace` - The log level for proto. Best to use `trace`.
- `PROTO_WASM_LOG=trace` - The log level for WASM plugins. Will write to a `<id>-debug.log` file in the current directory.
- `PROTO_DEBUG_COMMAND=true` - Print full `exec_command!` output to the console.
- `PROTO_CACHE=true` - Turn caching on or off.

## Tests

When a test is failing, we need to debug the result of a child process, as we execute `proto` as a child process to assert the entire CLI flow. An example test may look like this:

```rust
#[test]
fn installs_without_minor() {
    let sandbox = create_empty_sandbox();

    let mut cmd = create_proto_command(sandbox.path());
    cmd.arg("install")
        .arg("node")
        .arg("17")
        .arg("--")
        .arg("--no-bundled-npm")
        .assert()
        .success();

    assert!(sandbox.path().join("tools/node/17.9.1").exists());
}
```

> Note: Any test using `create_proto_command` will execute `proto` as a child process.

The `assert()` method is what executes the child process, waits, and returns the result. This is what we need to interact with to debug the test.

To start, comment out `success()` since it triggers a panic if the process is not succesful. You may need to replace it with an `assert!(false)` to force the test to keep failing (to view captured output).

We can then print stdout/stderr from the assert result like so:

```rust
#[test]
fn installs_without_minor() {
    let sandbox = create_empty_sandbox();

    let mut cmd = create_proto_command(sandbox.path());

    // assign to variable
    let assert = cmd
        .arg("install")
        .arg("node")
        .arg("17")
        .arg("--")
        .arg("--no-bundled-npm")
        .assert();
    // .success();

    // print captured output
    println!("{}", String::from_utf8_lossy(&assert.get_output().stdout));
    println!("{}", String::from_utf8_lossy(&assert.get_output().stderr));

    assert!(sandbox.path().join("tools/node/17.9.1").exists());

    // force test to fail
    assert!(false);
}
```

> When debugging a failing test, the WASM log file is written to the sandbox/fixture, and not to your current directory. To view the contents, read and print the file.
>
> `println!("{}", std::fs::read_to_string(sandbox.path().join("<id>-debug.log")).unwrap());`

# Debugging plugins

Debugging WASM plugins is non-trivial. I'll use the Node.js plugin as an example: https://github.com/moonrepo/tools

## Building

To start, build a debug target with `cargo wasi build` or `cargo build --target wasm32-wasi`. This will make it available at `target/wasm32-wasi/debug/<name>.wasm`.

To execute the debug `.wasm` file within proto, we need to configure a `.prototools` file that points to our newly built file, for example:

```toml
[plugins]
node-test = "file://./target/wasm32-wasi/debug/node_plugin.wasm"
```

We can then execute it with proto as such:

```shell
~/proto/target/debug/proto run node-test
```

## Logging

WASM plugins _cannot_ use the `println!` and `dbg!` macros, and must use the logging macros instead: `error!`, `warn!`, `info!`, `debug!`, and `log!`.

Once the macro statements have been added, and the `.wasm` file has been re-built, we can execute it with proto as such:

```shell
PROTO_LOG=trace PROTO_WASM_LOG=trace PROTO_CACHE=off ~/proto/target/debug/proto run node-test
```

This will create a `<id>-debug.log` file in the current directory with all log output from the WASM plugin and Extism runtime.

> When debugging a failing test, the WASM log file is written to `$CARGO_TARGET_DIR/wasm32-wasi/debug/<name>.log` instead.

# Debugging PDKs

proto has a few crates that are used directly by plugins: `proto_pdk`, `proto_pdk_api`, and `proto_pdk_test_utils`.

We can easily add debugging/logging to these crates, and test them within our plugins, by using the `path` setting in `Cargo.toml`. This will force Cargo to use our local crates, instead of the crates from crates.io.

For example in the Node.js plugin's [`Cargo.toml`](https://github.com/moonrepo/tools/blob/master/Cargo.toml), we can uncomment (or insert) the `path`s to point to crates in our local proto checkout:

```toml
proto_pdk = { version = "0.8.0", path = "../../proto/crates/pdk" }
proto_pdk_api = { version = "0.8.0", path = "../../proto/crates/pdk-api" }
proto_pdk_test_utils = { version = "0.8.2", path = "../../proto/crates/pdk-test-utils" }
```

From here, just re-build proto and the WASM plugin, and re-run the commands above.
