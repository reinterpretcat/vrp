# Python

The easiest way to run the solver from python is to use `subprocess` to run `vrp-cli`:

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