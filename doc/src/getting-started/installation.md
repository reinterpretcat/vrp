# Installation

VRP solver is built in Rust. To install it, either download a version from the [vrp releases](https://github.com/reinterpretcat/vrp/releases)
page, use `cargo install` or pull the source code from `master`.

## Install from source

Once pulled the source code, you can build it using `cargo`:

```bash
cargo build --release
```

Built binaries can be found in the `./target/release` directory.

## Install from Cargo

You can install vrp solver directly with `cargo install`:

```bash
cargo install vrp-cli
```

Ensure that your `$PATH` is properly configured to source the Crates binaries, and then run solver using the `vrp-cli` command.