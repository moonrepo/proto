{
  description = "A multi-language version manager, a unified toolchain";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      crane,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        devRustToolchain = rustToolchain.override {
          extensions = [ "rust-src" ];
          targets = [ "wasm32-wasip1" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        protoVersion = (builtins.fromTOML (builtins.readFile ./crates/cli/Cargo.toml)).package.version;

        source = pkgs.lib.fileset.toSource {
          root = ./.;
          fileset = pkgs.lib.fileset.unions [
            (craneLib.fileset.commonCargoSources ./.)
            ./crates/version-spec/src/syntax.pest
          ];
        };

        nativeDeps = with pkgs; [ pkg-config ];
        buildDeps =
          with pkgs;
          [ openssl ] ++ lib.optionals stdenv.hostPlatform.isDarwin [ libiconv ];

        commonArgs = {
          pname = "proto";
          version = protoVersion;
          src = source;

          cargoExtraArgs = "--locked --package proto_cli --bins";
          strictDeps = true;
          doCheck = false;

          nativeBuildInputs = nativeDeps;
          buildInputs = buildDeps;

          env.OPENSSL_NO_VENDOR = "1";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        proto = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
          }
        );
      in
      {
        packages = {
          inherit proto;
          proto-deps = cargoArtifacts;
          default = proto;
        };

        apps.default = {
          type = "app";
          program = "${proto}/bin/proto";
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = nativeDeps ++ [
            devRustToolchain
            pkgs.just
            pkgs.cargo-insta
            pkgs.cargo-nextest
            pkgs.cargo-wasi
          ];
          buildInputs = buildDeps;

          env = {
            OPENSSL_NO_VENDOR = "1";
            RUST_SRC_PATH = "${devRustToolchain}/lib/rustlib/src/rust/library";
          };

          shellHook = ''
            export WARPGATE_PLUGINS_DIR="$PWD/plugins"
          '';
        };
      }
    );
}
