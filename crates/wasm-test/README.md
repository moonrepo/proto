Build for testing:

```
cargo install cargo-wasi
cargo wasi build -p proto_test_plugin

# Old
cargo build --target wasm32-wasi -p proto_test_plugin
cargo build --target wasm32-unknown-unknown -p proto_test_plugin
```
