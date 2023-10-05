# Project structure

The project consists of the main and auxiliary crates. Additionally, there is some logical separation inside each group.

## Main crates

The following crates are "the heart" of VRP solver:

* [`rosomaxa`](https://docs.rs/rosomaxa/latest): contains key algorithms for solving optimization problems __without__
  locking to the VRP domain such as hyper heuristics, evolution strategies, etc.
* [`vrp_core`](https://docs.rs/vrp_core/latest): this crate provides all core VRP models with various meta heuristics to
  solve rich VRP
* [`vrp_scientific`](https://docs.rs/vrp_scientific/latest): has a building blocks to solve problems from some of scientific
  benchmarks. It is useful to evaluate the solver performance in terms of solution quality, search stability and running time.
* [`vrp_pragmatic`](https://docs.rs/vrp_pragmatic/latest): provides models and features to support rich VRP. It includes:
  * pragmatic model, serializable in json
  * solution checker
  * problem validator
* [`vrp_cli`](https://docs.rs/vrp_cli/latest): exposes VRP solve as command line interface or static library. Additionally,
  has some extra features, such as:
  * various extra commands
  * pyO3 bindings to make library usable from Python
  * WASM bindings to run solver directly in the browser
  * ..

For these crates, you can find extra information normally published on docs.rs.

## Helper crates/functionality

There are few:

* `experiments/heuristic-research`: my way to experiment with heuristic using some hooks and visualizations.
   Live version is exposed [here](https://reinterpretcat.github.io/heuristics/www/)
* `examples/json-pragmatic`: provides example how to use the library as a crate + contains tests and benchmarks on test data
* `examples/jvm-interop` / `python-interop`: some examples how to call library from other languages
* `examples/data`: various examples of problem definitions. Mostly used for testing and documentation