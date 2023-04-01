# Installation

Depending on your development environment, you can use different ways to install the solver:

## Install with Python

The functionality of `vrp-cli` is published to [pypi.org](https://pypi.org/project/vrp-cli/), so you can just install it
using pip and use from python:

```shell
pip install vrp-cli
python examples/python-interop/example.py # run test example
```

Alternatively, you can use [maturin](https://github.com/PyO3/maturin) tool to build solver locally.

You can find extra information in [python example section](https://reinterpretcat.github.io/vrp/examples/interop/python.html)
of the docs. The [full source code](./examples/python-interop/example.py) of python example is available in the repo which
contains useful model wrappers with help of `pydantic` lib.


## Install from Docker

Another fast way to try vrp solver on your environment is to use `docker` image (not performance optimized):

* **run public image** from `Github Container Registry`:

```bash
    docker run -it -v $(pwd):/repo --name vrp-cli --rm ghcr.io/reinterpretcat/vrp/vrp-cli:1.20.0
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