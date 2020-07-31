[![](https://docs.rs/vrp-core/badge.svg)](https://docs.rs/vrp-core)
[![crates.io](https://img.shields.io/crates/v/vrp-cli.svg)](https://crates.io/crates/vrp-cli)
![build](https://github.com/reinterpretcat/vrp/workflows/build/badge.svg)

![VRP example](docs/resources/vrp-example.png "VRP with Route Balance")

# Description

This project provides the way to solve multiple variations of **Vehicle Routing Problem** known as rich VRP. It provides
default metaheuristic implementation which can be roughly described as
`Multi-objective Parthenogenesis based Evolutionary Algorithm with Ruin and Recreate Mutation Operator`


# Getting started

Please check [A Vehicle Routing Problem Solver Documentation](https://reinterpretcat.github.io/vrp).


# Design goal

Although performance is constantly in focus, the main idea behind design is extensibility: the project
aims to support a wide range of VRP variations known as Rich VRP. This is achieved through various extension
points: custom constraints, objective functions, acceptance criteria, etc.


# How to use

VRP solver is built in Rust. To install it, use `cargo install` or pull the source code from `master`.


## Install from source

Once pulled the source code, you can build it using `cargo`:

```bash
cargo build --release
```
Built binaries can be found in the `./target/release` directory.

Alternatively, you can try to run the following script from the project root:

        ./solve_problem.sh examples/data/pragmatic/objectives/berlin.default.problem.json

It will build the executable and automatically launch the solver with the specified VRP definition. Results are
stored in the folder where a problem definition is located.

## Install from Cargo

You can install vrp solver `cli` tool directly with `cargo install`:

```bash
cargo install vrp-cli
```

Ensure that your `$PATH` is properly configured to source the crates binaries, and then run solver using the `vrp-cli` command.


## Use from command line

`vrp-cli` crate is designed to use on problems defined in scientific or custom (aka 'pragmatic') format:

```bash
vrp-cli solve pragmatic problem_definition.json -m routing_matrix.json --max-time=120
```

Please refer to crate docs for more details.


## Use from code

If you're using rust, then you can simply use `vrp-scientific`, `vrp-pragmatic` crates to solve VRP problem
defined in 'pragmatic' or 'scientific' format using default metaheuristic. For more complex scenarios, please refer to
`vrp-core` documentation.

If you're using some other language, e.g java, kotlin, javascript, please check `examples` section to see how to call
the library from it.


# Project structure

The project consists of the following parts:
- **vrp solver code**: the source code of the solver is split into four crates:
    - *vrp-core*: a core crate with default metaheuristic implementation
    - *vrp-scientific*: a crate with functionality to solve problems from some of scientific benchmarks on top of the core crate
    - *vrp-pragmatic*: a crate which provides logic to solve rich VRP using `pragmatic` json format on top of the core crate
    - *vrp-cli*: a crate which aggregates logic of others crates and exposes them as a library and application
- **docs**: a source code of the user guide documentation published [here](https://reinterpretcat.github.io/vrp).
    Use [mdbook](https://github.com/rust-lang/mdBook) tool to build it locally.
- **examples**: provides various examples:
    - *data*: a data examples such as problem definition, configuration, etc.
    - *json-pragmatic*: an example how to solve problem in `pragmatic` json format from rust code using the project crates
    - *jvm-interop*: a gradle project which demonstrates how to use the library from java and kotlin

# Dependant projects

* [analysis](https://github.com/reinterpretcat/vrp-analysis): provides way to analyze solutions, algorithm behaviour (WIP)
* [api](https://github.com/reinterpretcat/vrp-api): API prototype built using Rust/AWS/Terraform (PoC)

# Status

Experimental.