# Contributing to Proto

We're glad you're interested in contributing to Proto! This document will provide you with all the information you need to get started.

## Code of Conduct

We expect all contributors to adhere to the highest standards of behavior. Respect for others and professionalism are paramount. Any form of harassment, bullying, or discrimination will not be tolerated.

## Reporting Issues

If you encounter a bug or have a feature request, please first check if the issue has already been reported. If not, create a new issue in the repository. Please provide as much detail as possible so we can understand and reproduce the issue. You can use the bug report template provided in the repository.

## Pull Requests

We welcome pull requests! If you're fixing a bug or implementing a feature, please first open an issue describing the problem or feature. Then you can submit a pull request linked to that issue.

Before submitting your pull request, please ensure your code adheres to the following guidelines:

- Your code should be formatted according to the Rust style guide.
- Your code should pass all tests. You can run tests with `cargo test`.
- If you're adding new functionality, please also add corresponding tests.
- If you're making a significant change, please update the documentation accordingly.

## Development Environment

You'll need the following tools installed in your development environment:

- Rust: The project uses Rust as the primary programming language. You can install Rust from the [official website](https://www.rust-lang.org/tools/install).
- Cargo: Cargo is the Rust package manager, which is used for managing Rust dependencies, building the project, and running tests.

The project uses a specific version of the Rust toolchain. You can install the correct version using `rustup`:

```bash
rustup override set $(cat rust-toolchain.toml | grep channel | cut -d' ' -f3)
```

## Building the Project

You can build the project using Cargo:

```bash
cargo build
```

## Running Tests

You can run the tests using Cargo:

```bash
cargo test
```

## Contact

If you have any questions or need help, you can reach out to us on our [Discord server](https://discord.gg/qCh9MEynv2).

Thank you for your interest in contributing to Proto!
