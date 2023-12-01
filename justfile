init:
	cargo install cargo-binstall
	cargo binstall cargo-insta cargo-nextest cargo-wasi

build:
	cargo build --workspace

build-wasm:
	cd plugins && cargo wasi build

format:
	cargo fmt --all

format-check:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets

lint-wasm:
	cd plugins && cargo clippy --workspace --all-targets

test name="":
	cargo nextest run --workspace {{name}}

test-ci:
	cargo nextest run --workspace --exclude proto_pdk --profile ci --config-file ./.cargo/nextest.toml

setup-shims:
	cargo build --workspace
	mkdir -p ./shims
	cp -f ~/.cargo/shared-target/debug/proto-shim ./shims/node
	ln -f ~/.cargo/shared-target/debug/proto-shim ./shims/node-hard
	ln -fs ~/.cargo/shared-target/debug/proto-shim ./shims/node-soft
