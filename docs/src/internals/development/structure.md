# Project structure

The project consists of the main and auxiliary crates. Additionally, there is some logical separation inside each group.

## Main crates

The following crates are "the heart" of VRP solver:

* [`rosomaxa`](https://docs.rs/rosomaxa/latest): contains key algorithms for solving optimization problems __without__
  locking to the VRP domain such as hyper heuristics, evolution strategies, etc.
* [`vrp_core`](https://docs.rs/vrp_core/latest): this crate provides all core VRP models / features with various meta heuristics to
  solve rich VRP
* [`vrp_scientific`](https://docs.rs/vrp_scientific/latest): has a building blocks to solve problems from some of scientific
  benchmarks. It is useful to evaluate the solver performance in terms of solution quality, search stability and running time.
* [`vrp_pragmatic`](https://docs.rs/vrp_pragmatic/latest): provides models to support rich VRP. It includes:
  * pragmatic model, serializable in json
  * solution checker
  * problem validator
* [`vrp_cli`](https://docs.rs/vrp_cli/latest): exposes VRP solve as command line interface or static library. Additionally,
  has some extra features, such as:
  * various extra commands
  * language bindings in `vrp-cli/src/interop`: a C ABI, pyO3 bindings for Python and WASM bindings for javascript.
    All three are thin adapters over one shared implementation, so they expose the same operations with the same
    semantics and the same error representation
  * ..

For these crates, you can find extra information normally published on docs.rs.

## Helper crates/functionality

There are few:

* `experiments/heuristic-research`: my way to experiment with heuristic using some hooks and visualizations.
   Live version is exposed [here](https://reinterpretcat.github.io/heuristics/www/)
* `examples/json-pragmatic`: provides example how to use the library as a crate + contains tests and benchmarks on test data
* `examples/jvm-interop` / `python-interop` / `js-interop`: some examples how to call library from other languages
* `examples/data`: various examples of problem definitions. Mostly used for testing and documentation

## Language bindings

The rust side lives in `vrp-cli/src/interop`, where a C ABI, pyO3 bindings for Python and WASM bindings for javascript
are thin adapters over one shared implementation, so all three expose the same operations with the same semantics and the
same error representation.

Everything those bindings are built from lives in `vrp-cli/bindings`: the json schemas derived from the rust types, the
`pydantic` models and typescript declarations generated from those schemas, and the single script which regenerates all
of it. Generated output is not committed: CI and release workflows generate it from the rust source of truth and pass it
to the jobs that consume it. See `vrp-cli/bindings/README.md`.

It sits inside the crate rather than next to the examples because two of the artifacts are build inputs: `maturin` builds
the wheel from `bindings/python`, and the wasm binding embeds `bindings/typescript/format_types.d.ts` with `include_str!`.
The release workflow generates the declaration before packaging, and the crate manifest includes it in the published
archive even though it is gitignored.
