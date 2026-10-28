# Python

## Using pip

This is the easiest way to start using the solver's latest version:

```shell
pip install vrp-cli
python examples/python-interop/example.py # test example
```

See python code example in repo or in next section.


## Models

The package ships typed [pydantic](https://docs.pydantic.dev/) models for every document the solver exchanges, so there
is no need to write them by hand:

| module                    | describes                                    |
|---------------------------|----------------------------------------------|
| `vrp_cli.models.problem`  | a problem definition in `pragmatic` format   |
| `vrp_cli.models.matrix`   | a routing matrix                             |
| `vrp_cli.models.config`   | the solver configuration                     |
| `vrp_cli.models.solution` | a solution as produced by the solver         |
| `vrp_cli.models.error`    | one entry of a reported failure              |

They are generated from the rust types the solver itself uses, so they cannot describe a format it does not accept, and
a missing required field or an unknown objective is reported by `pydantic` before the solver is called. Models serialize
with the json field names by default, which means `model_dump_json()` produces a document the solver accepts directly.
Use `exclude_none=True` to omit optional fields rather than sending nulls.

Each model carries the documentation of the corresponding rust type, so `help(Job)` and editor tooltips describe the
fields.


## Using maturin

You can use [maturin](https://github.com/PyO3/maturin) tool to build solver locally for you. Here are the steps:

1. From the repository root, create a virtual environment and install maturin and the pinned code
   generators:
    ```shell
    python3 -m venv vrp-cli/.venv
    source vrp-cli/.venv/bin/activate
    pip install -U pip maturin[patchelf]
    pip install -r vrp-cli/bindings/python/requirements-codegen.txt
    npm ci --prefix vrp-cli/bindings/typescript
    ```

2. Generate the ignored models from the rust types, then use maturin to build and install the solver
   library in your current environment. The `py_bindings` feature is declared in `pyproject.toml`, so
   it does not need to be passed here:
    ```shell
    ./vrp-cli/bindings/generate.sh
    cd vrp-cli
    maturin develop --release
    ```

3. Import and use the library in your python code:

```python
{{#include ../../../../examples/python-interop/example.py}}
```

You can check the project repository for complete example, including an
[interactive tutorial](https://github.com/reinterpretcat/vrp/tree/master/examples/python-interop/tutorial.ipynb).


## Error handling

Every function raises `OSError` whose message is a json array of errors, each with a `code`, a `cause` and a suggested
`action`, so failures can be inspected programmatically:

```python
import json
import vrp_cli
from vrp_cli.models import FormatError

try:
    vrp_cli.solve_pragmatic(problem=problem_json, matrices=[], config=config_json)
except OSError as err:
    for error in (FormatError.model_validate(item) for item in json.loads(str(err))):
        print(error.code, error.cause, error.action)
```

See the [error index](../../concepts/pragmatic/errors/index.md) for the meaning of each code. Note that
`solve_pragmatic` validates the problem before solving it.


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
