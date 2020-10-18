# Installation

## Install from Docker

The fastest way to get vrp solver on your environment is to use public `docker` image from `Github Container Registry`:

```bash
    docker run -it -v $(pwd):/repo --name vrp-cli --rm ghcr.io/reinterpretcat/vrp/vrp-cli:1.6.2
```

## Install from source

VRP solver is built in Rust. You would need to install `cargo` to built it:

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