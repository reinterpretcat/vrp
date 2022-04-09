import os
import json
import re
import shutil
import subprocess
import pathlib
import tempfile
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

    def solve_pragmatic(self, problem_path, matrices, config_path, solution_path, extra_args):
        matrices_args = sum(list(map(lambda m: ['-m', m], matrices)), [])
        command = sum([[self.cli_path], ['solve'], ['pragmatic'], [problem_path], matrices_args,
                       ['-c'], [config_path], ['-o'], [solution_path]], [])

        if extra_args != '':
            command = command + [extra_args]

        p = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

        if p.returncode == 0:
            with open(solution_path, 'r') as f:
                solution_str = f.read()
                return json.loads(solution_str, object_hook=Deserializer.from_dict)
        else:
            raise ValueError("cannot solve problem: {}".format(p.stderr))
            pass

    def solve_scientific(self, scientific_format, problem_path, config_path, solution_path, extra_args):
        command = [self.cli_path, 'solve', scientific_format, problem_path, '-c', config_path, '-o', solution_path]

        if extra_args != '':
            command = command + [extra_args]

        p = subprocess.run(command, capture_output=True, text=True)

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
                return f.read(), float(statistic.group('duration')), int(statistic.group('generations')), \
                       float(statistic.group('speed')), float(best.group('cost')), int(best.group('tours')), \
                       int(best.group('unassigned'))
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


def prepare_environment(root_temp_dir, experiment_config_url):
    print("setup temporary directory in '{}'".format(root_temp_dir))
    experiment_config_path = "{}/config.json".format(root_temp_dir)

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

    return solver_config_root, experiment_config, solver_versions
