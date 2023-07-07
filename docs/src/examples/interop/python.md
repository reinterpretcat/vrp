# Python

## Using pip

This is the easiest way to start using the solver's latest version:

```shell
pip install vrp-cli
python examples/python-interop/example.py # test example
```

See python code example in repo or in next section.


## Using maturin

You can use [maturin](https://github.com/PyO3/maturin) tool to build solver locally for you. Here are the steps:

1. Create a virtual environment and install maturin (and pydantic):
    ```shell
    cd vrp-cli # directory of the crate where with python bindings are located
    python3 -m venv .venv
    source .venv/bin/activate
    pip install -U pip maturin[patchelf] pydantic
    pip freeze
    ```

2. Use maturin to build and install the solver library in your current environment:
    ```shell
    maturin develop --release --features "py_bindings"
    ```

3. Import and use the library in your python code:

```python
{{#include ../../../../examples/python-interop/example.py}}
```

You can check the project repository for complete example.

**Please note**, that type wrappers, defined in examples with `pydantic`, are incomplete. However, it should be enough to
get started, and you can tweak them according to the documentation or rust source code.


## Using local build

Another way to run the solver, built locally, from python is to use `subprocess` to run `vrp-cli` directly:

```python
import subprocess
import json

# NOTE: ensure that paths are correct on your environment
cli_path = "./target/release/vrp-cli"
problem_path = "./examples/data/pragmatic/simple.basic.problem.json"
solution_path = "./examples/data/pragmatic/simple.basic.solution.json"
geojson_solution_path = "./examples/data/pragmatic/simple.basic.solution.geojson"

class Deserializer:
    @classmethod
    def from_dict(cls, dict):
        obj = cls()
        obj.__dict__.update(dict)
        return obj

class SolverClient:
    def __init__(self, cli_path):
        self.cli_path = cli_path

    def solve_pragmatic(self, problem_path, solution_path, geojson_solution_path):
        # NOTE: modify example to pass matrix, config, initial solution, etc.
        p = subprocess.run([self.cli_path, 'solve', 'pragmatic', problem_path,
            '-o', solution_path, '-g', geojson_solution_path, '--log'])

        if p.returncode == 0:
            with open(solution_path, 'r') as f:
                solution_str = f.read()
                return json.loads(solution_str, object_hook=Deserializer.from_dict)
        else:
            pass

solver = SolverClient(cli_path)
solution = solver.solve_pragmatic(problem_path, solution_path, geojson_solution_path)

print(f"Total cost is {solution.statistic.cost}, tours: {len(solution.tours)}")
```

**Please note**, that the solver expects file paths instead of json strings as input arguments.