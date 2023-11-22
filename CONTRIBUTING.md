# Contributing to proto

We're glad you're interested in contributing to proto! This document will provide you with all the information you need to get started.

## Code of Conduct

We expect all contributors to adhere to the highest standards of behavior. Respect for others and professionalism are paramount. Any form of harassment, bullying, or discrimination will not be tolerated.

## Reporting issues

If you encounter a bug or have a feature request, please first check if the issue has already been reported. If not, create a new issue in the repository. Please provide as much detail as possible so we can understand and reproduce the issue. You can use the bug report template provided in the repository.

> If you're reporting an issue for a WASM plugin, please report it in the [appropriate plugin repository](https://github.com/orgs/moonrepo/repositories?q=plugin&type=all&language=&sort=).

## Pull requests

We welcome pull requests! If you're fixing a bug or implementing a feature, please first open an issue describing the problem or feature. Then you can submit a pull request linked to that issue.

Before submitting your pull request, please ensure your code adheres to the following guidelines:

- Your code should be formatted according to the Rust style guide.
- Your code should pass all tests and lints.
- If you're adding new functionality, please also add corresponding tests.
- If you're making a significant change, please update the documentation accordingly.

## Development environment

You'll need the following tools installed in your development environment:

- Rust: The project uses Rust as the primary programming language. You can install Rust from the [official website](https://www.rust-lang.org/tools/install).
- Cargo: Cargo is the Rust package manager, which is used for managing Rust dependencies, building the project, and running tests.

Furthermore, we make use of just, insta, nextest, and wasi, which can be installed with:

```bash
# cargo install
cargo binstall just cargo-insta cargo-nextest cargo-wasi
```

Or if you already have `just` installed:

```bash
just init
```

## Building the project

You can build the project using Cargo:

```bash
just build
```

## Running tests

You can run the tests using Cargo:

```bash
just test

# To run matching tests
just test <name>
```

## Contact

If you have any questions or need help, you can reach out to us on our [Discord server](https://discord.gg/qCh9MEynv2).

Thank you for your interest in contributing to proto!
