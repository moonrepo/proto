set windows-shell := ["pwsh.exe", "-Command"]

init:
	cargo install cargo-binstall
	cargo binstall cargo-insta cargo-nextest cargo-wasi

build:
	cargo build --bin proto --bin proto-shim --no-default-features

build-shim:
	cargo build --bin proto-shim

build-wasm:
	cd plugins && cargo build --workspace --target wasm32-wasip1 --release

check:
	cargo check --workspace

format:
	cargo fmt --all

format-check:
	cargo fmt --all --check

gen:
	cargo run -p proto_codegen

lint:
	cargo clippy --workspace --all-targets

lint-wasm:
	cd plugins && cargo clippy --workspace --all-targets

mcp:
	PROTO_NPM_VERSION=* npx @modelcontextprotocol/inspector -- cargo run -- mcp

test name="":
	just build
	cargo nextest run --workspace {{name}}

test-ci:
	cargo nextest run --workspace --exclude proto_pdk --profile ci --config-file ./.cargo/nextest.toml

setup-shims:
	cargo build --workspace
	mkdir -p ./shims
	cp -f "$CARGO_TARGET_DIR/debug/proto-shim" ./shims/node
	ln -f "$CARGO_TARGET_DIR/debug/proto-shim" ./shims/node-hard
	ln -fs "$CARGO_TARGET_DIR/debug/proto-shim" ./shims/node-soft

setup-shims-win:
	cargo build --workspace
	New-Item -ItemType Directory -Force -ErrorAction SilentlyContinue shims
	New-Item -ItemType HardLink -Force -Name "shims\node-hard.exe" -Value "target\debug\proto-shim.exe"
	New-Item -ItemType SymbolicLink -Force -Name "shims\node-soft.exe" -Value "target\debug\proto-shim.exe"
	Copy-Item "target\debug\proto-shim.exe" -Destination "shims\node.exe"
