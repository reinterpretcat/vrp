[![](https://docs.rs/vrp-core/badge.svg)](https://docs.rs/vrp-core)
[![crates.io](https://img.shields.io/crates/v/vrp-cli.svg)](https://crates.io/crates/vrp-cli)
![build](https://github.com/reinterpretcat/vrp/workflows/build/badge.svg)
[![codecov](https://codecov.io/gh/reinterpretcat/vrp/branch/master/graph/badge.svg)](https://codecov.io/gh/reinterpretcat/vrp)
[![DOI](https://zenodo.org/badge/238436117.svg)](https://zenodo.org/badge/latestdoi/238436117)

![VRP example](docs/resources/vrp-example.png "VRP with Route Balance")

# Description

This project provides the way to solve multiple variations of **Vehicle Routing Problem** known as rich VRP. It provides
custom hyper- and meta-heuristic implementations, shortly described [here](https://reinterpretcat.github.io/vrp/internals/index.html).

If you use the project in academic work, please consider citing:

```
@misc{builuk_rosomaxa_2021,
    author       = {Ilya Builuk},
    title        = {{A new solver for rich Vehicle Routing Problem}},
    year         = 2021,
    doi          = {10.5281/zenodo.4624037},
    publisher    = {Zenodo},
    url          = {https://doi.org/10.5281/zenodo.4624037}
}
```

# Design goal

Although performance is constantly in focus, the main idea behind design is extensibility: the project
aims to support a wide range of VRP variations known as Rich VRP. This is achieved through various extension
points: custom constraints, objective functions, acceptance criteria, etc.


# Getting started

For general installation steps and basic usage options, please check next sections. More detailed overview of features
is presented in [A Vehicle Routing Problem Solver Documentation](https://reinterpretcat.github.io/vrp).


# Installation

You can install vrp solver using three different ways:

## Install from Docker

The fastest way to try vrp solver on your environment is to use `docker` image (not performance optimized):

* **run public image** from `Github Container Registry`:

```bash
    docker run -it -v $(pwd):/repo --name vrp-cli --rm ghcr.io/reinterpretcat/vrp/vrp-cli:1.10.3
```

* **build image locally** using `Dockerfile` provided:

```bash
docker build -t vrp_solver .
docker run -it -v $(pwd):/repo --rm vrp_solver
```

Please note that the docker image is built using `musl`, not `glibc` standard library. So there might be some performance
implications.

## Install from Cargo

You can install vrp solver `cli` tool directly with `cargo install`:

    cargo install vrp-cli

Ensure that your `$PATH` is properly configured to source the crates binaries, and then run solver using the `vrp-cli` command.

## Install from source

Once pulled the source code, you can build it using `cargo`:

    cargo build --release

Built binaries can be found in the `./target/release` directory.

Alternatively, you can try to run the following script from the project root:

    ./solve_problem.sh examples/data/pragmatic/objectives/berlin.default.problem.json

It will build the executable and automatically launch the solver with the specified VRP definition. Results are
stored in the folder where a problem definition is located.


# Usage

Use can use vrp solver either from command line or from code:

## Use from command line

`vrp-cli` crate is designed to use on problems defined in scientific or custom json (aka `pragmatic`) format:

    vrp-cli solve pragmatic problem_definition.json -m routing_matrix.json --max-time=120

Please refer to [getting started](https://reinterpretcat.github.io/vrp/getting-started/index.html) section in
the documentation for more details.

## Use from code

If you're using rust, then you can simply use `vrp-scientific`, `vrp-pragmatic` crates to solve VRP problem
defined in `pragmatic` or `scientific` format using default metaheuristic. For more complex scenarios, please refer to
`vrp-core` documentation.

If you're using some other language, e.g java, kotlin, javascript, python, please check
[interop](https://reinterpretcat.github.io/vrp/examples/interop/index.html) section in documentation examples to see how
to call the library from it.


# Project structure

The project consists of the following parts:
- **vrp solver code**: the source code of the solver is split into four crates:
    - *vrp-core*: a core crate with various metaheuristic building blocks and its default implementation
    - *vrp-scientific*: a crate with functionality to solve problems from some of scientific benchmarks on top of the core crate
    - *vrp-pragmatic*: a crate which provides logic to solve rich VRP using `pragmatic` json format on top of the core crate
    - *vrp-cli*: a crate which aggregates logic of others crates and exposes them as a library and application
- **docs**: a source code of the user guide documentation published [here](https://reinterpretcat.github.io/vrp).
    Use [mdbook](https://github.com/rust-lang/mdBook) tool to build it locally.
- **examples**: provides various examples:
    - *data*: a data examples such as problem definition, configuration, etc.
    - *json-pragmatic*: an example how to solve problem in `pragmatic` json format from rust code using the project crates
    - *jvm-interop*: a gradle project which demonstrates how to use the library from java and kotlin


# Status

Experimental.
