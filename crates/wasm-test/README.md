Build for testing:

```
cargo install cargo-wasi
cargo wasi build -p proto_wasm_test

# Old
cargo build --target wasm32-wasi -p proto_wasm_test
```
