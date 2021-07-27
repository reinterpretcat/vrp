import csv
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import pathlib
import urllib
import urllib.request
from io import BytesIO
from zipfile import ZipFile


class Deserializer:
    @classmethod
    def from_dict(cls, dict):
        obj = cls()
        obj.__dict__.update(dict)
        return obj


class SolverClient:
    def __init__(self, cli_path):
        self.cli_path = cli_path

    def solve_pragmatic(self, problem_path, matrices, config_path, solution_path):
        matrices_args = sum(list(map(lambda m: ['-m', m], matrices)), [])
        command = sum([[self.cli_path], ['solve'], ['pragmatic'], [problem_path], matrices_args,
                       ['-c'], [config_path], ['-o'], [solution_path]], [])

        p = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

        if p.returncode == 0:
            with open(solution_path, 'r') as f:
                solution_str = f.read()
                return json.loads(solution_str, object_hook=Deserializer.from_dict)
        else:
            raise ValueError("cannot solve problem: {}".format(p.stderr))
            pass

    def solve_solomon(self, problem_path, config_path, solution_path):
        p = subprocess.run([self.cli_path, 'solve', 'solomon', problem_path, '-c', config_path, '-o', solution_path],
                           capture_output=True, text=True)

        if p.returncode == 0:
            # expected: [10s] total generations: 471, speed: 46.13 gen/sec
            for statistic in re.finditer(
                    r"\[(?P<duration>[0-9]+)s] total generations: (?P<generations>[0-9]+), speed: (?P<speed>[0-9.]+)",
                    p.stdout):
                pass

            try:
                statistic
            except NameError:
                raise ValueError('cannot get solution statistic')

            # expected: rank: 0, cost: 57137.26(0.000%), tours: 94, unassigned: 0, fitness: (0.000, 94.000, 57137.259)
            for best in re.finditer(
                    r"rank: 0, cost: (?P<cost>[0-9.]+)[^,]+, tours: (?P<tours>[0-9]+), unassigned: (?P<unassigned>[0-9]+)",
                    p.stdout):
                pass

            try:
                best
            except NameError:
                raise ValueError('cannot get best statistic')

            with open(solution_path, 'r') as f:
                return f.read(), statistic.group('duration'), statistic.group('generations'), statistic.group('speed'), \
                       best.group('cost'), best.group('tours'), best.group('unassigned')
        else:
            raise ValueError("cannot solve problem: {}".format(p.stderr))
            pass


def download_and_extract(url, extract_to):
    pathlib.Path(extract_to).mkdir(parents=True, exist_ok=True)
    http_response = urllib.request.urlopen(url)

    zipfile = ZipFile(BytesIO(http_response.read()))
    zipfile.extractall(path=extract_to)


def build_solver(source_url, destination_path):
    print('try to build: ', source_url)
    url, branch = source_url.split('@')

    with tempfile.TemporaryDirectory() as temp_dir:
        subprocess.run(['git', 'clone', '-b', branch, url, temp_dir])
        subprocess.run(['cargo', 'build', '-p', 'vrp-cli', '--release'], cwd=temp_dir)
        os.makedirs(destination_path, exist_ok=True)
        shutil.copy("{}/target/release/vrp-cli".format(temp_dir), destination_path)


def prepare_solver_versions(root_path, versions_meta):
    versions = []
    for version_meta in versions_meta:
        solver_path = "{}/{}".format(root_path, version_meta.name)
        build_solver(version_meta.url, solver_path)
        versions.append(Deserializer().from_dict({
            'name': version_meta.name,
            'client': SolverClient("{}/vrp-cli".format(solver_path)),
            'path': solver_path
        }))

    return versions


if len(sys.argv) < 2:
    print("Provide url to experiment config")
    sys.exit(1)
else:
    experiment_config_url = sys.argv[1]

with tempfile.TemporaryDirectory() as root_temp_dir:
    print("setup temporary directory in '{}'".format(root_temp_dir))
    experiment_config_path = "{}/config.json".format(root_temp_dir)
    experiment_output_path = "{}/experiments-results.csv".format(os.path.dirname(os.path.realpath(__file__)))

    print('downloading experiment config..')
    urllib.request.urlretrieve(experiment_config_url, experiment_config_path)

    with open(experiment_config_path, 'r') as f:
        experiment_config_content = f.read()
        experiment_config = json.loads(experiment_config_content, object_hook=Deserializer.from_dict)

    print('downloading solver configs..')
    solver_config_root = "{}/solver-config".format(root_temp_dir)
    download_and_extract(experiment_config.data.config.url, solver_config_root)

    print('preparing solver clis..')
    solver_versions = prepare_solver_versions(root_temp_dir, experiment_config.versions)

    with open(experiment_output_path, mode='w') as best_known_file:
        best_known_writer = csv.writer(best_known_file, delimiter=',', quotechar='"', quoting=csv.QUOTE_MINIMAL)
        best_known_writer.writerow(
            ["Version", "Config", "Problem", "Type", "Iteration", "Duration", "Generations", "Speed", "Cost", "Tours",
             "Unassigned"])

        for iteration_number in range(experiment_config.parameters.iterations):
            print("iteration {}".format(iteration_number))
            for solver_config in experiment_config.data.config.files:
                solver_config_path = "{}/{}".format(solver_config_root, solver_config.path)

                if hasattr(experiment_config.data, 'pragmatic'):
                    print('processing pragmatic files..')
                    pragmatic_root = "{}/pragmatic".format(root_temp_dir)
                    download_and_extract(experiment_config.data.pragmatic.url, pragmatic_root)

                    for pragmatic_problem in experiment_config.data.pragmatic.files:
                        pragmatic_name = pragmatic_problem.name
                        pragmatic_path = "{}/{}".format(pragmatic_root, pragmatic_problem.path)
                        pragmatic_matrices = pragmatic_problem.matrices

                        for solver_cli in solver_versions:
                            pragmatic_solution_path = "{}/pragmatic_{}_solution_{}_{}.json".format(
                                solver_cli.path, pragmatic_name, solver_config.name, iteration_number)
                            pragmatic_solution = solver_cli.client.solve_pragmatic(pragmatic_path, pragmatic_matrices,
                                                                                   solver_config_path,
                                                                                   pragmatic_solution_path)

                            best_known_writer.writerow([
                                solver_cli.name, solver_config.name, pragmatic_name, 'pragmatic', iteration_number,
                                pragmatic_solution.extras.metrics.duration,
                                pragmatic_solution.extras.metrics.generations,
                                "{:.4f}".format(round(pragmatic_solution.extras.metrics.speed, 2)),
                                "{:.4f}".format(round(pragmatic_solution.statistic.cost, 2)),
                                len(pragmatic_solution.tours), len(getattr(pragmatic_solution, "unassigned", []))
                            ])

                if hasattr(experiment_config.data, 'solomon'):
                    print('processing solomon files..')
                    solomon_root = "{}/solomon".format(root_temp_dir)
                    download_and_extract(experiment_config.data.solomon.url, solomon_root)

                    for solomon_problem in experiment_config.data.solomon.files:
                        solomon_name = solomon_problem.name
                        solomon_path = "{}/{}".format(solomon_root, solomon_problem.path)

                        for solver_cli in solver_versions:
                            solomon_solution_path = "{}/solomon_{}_solution_{}_{}.txt".format(
                                solver_cli.path, solomon_name, solver_config.name, iteration_number)
                            solomon_solution, duration, generations, speed, cost, tours, unassigned = \
                                solver_cli.client.solve_solomon(solomon_path, solver_config_path, solomon_solution_path)

                            best_known_writer.writerow([
                                solver_cli.name, solver_config.name, solomon_name, 'solomon', iteration_number,
                                duration, generations, speed, cost, tours, unassigned
                            ])
