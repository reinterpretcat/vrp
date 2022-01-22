# Internals

This chapter describes some internals of the project.

## Project structure

The project consists of the following parts:

- **vrp solver code**: the source code of the solver is split into four crates:
    - *rosomaxa*: a crate with various metaheuristic building blocks and its default implementation
    - *vrp-core*: a core crate for vrp domain
    - *vrp-scientific*: a crate with functionality to solve problems from some of scientific benchmarks on top of the core crate
    - *vrp-pragmatic*: a crate which provides logic to solve rich VRP using `pragmatic` json format on top of the core crate
    - *vrp-cli*: a crate which aggregates logic of others crates and exposes them as a library and application
- **docs**: a source code of the user guide documentation published [here](https://reinterpretcat.github.io/vrp).
  Use [mdbook](https://github.com/rust-lang/mdBook) tool to build it locally.
- **examples**: provides various examples:
    - *data*: a data examples such as problem definition, configuration, etc.
    - *json-pragmatic*: an example how to solve problem in `pragmatic` json format from rust code using the project crates
    - *jvm-interop*: a gradle project which demonstrates how to use the library from java and kotlin
