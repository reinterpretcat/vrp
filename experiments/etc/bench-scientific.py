import csv
import os
import shutil
import sys
import tempfile
from shared import download_and_extract, prepare_environment

if len(sys.argv) < 2:
    print("Provide url to experiment config")
    sys.exit(1)
else:
    experiment_config_url = sys.argv[1]


def get_best_known_cost(problem_path):
    best_known_solution_path = "{}.sol".format(os.path.splitext(problem_path)[0])

    file = open(best_known_solution_path, 'r')
    lines = file.readlines()
    for line in lines:
        if line.startswith("Cost"):
            return float(line.split(' ')[1])

    return -1


with tempfile.TemporaryDirectory() as root_temp_dir:
    solver_config_root, experiment_config, solver_versions = prepare_environment(root_temp_dir, experiment_config_url)

    solver_config = experiment_config.data.config.files[0]
    solver_cli = solver_versions[0]

    print("NOTE: use only the first config: '{}'".format(solver_config.name))
    print("NOTE: use only the first solver version: '{}'".format(solver_cli.name))
    print("NOTE: ignore iterations parameter: {}".format(experiment_config.parameters.iterations))

    solver_config_path = "{}/{}".format(solver_config_root, solver_config.path)
    results_output_path = ("{}/results".format(os.path.dirname(os.path.realpath(__file__))))
    csv_output_path = "{}/scientific-bench-results.csv".format(results_output_path)

    os.mkdir(results_output_path)

    with open(csv_output_path, mode='w') as best_known_file:
        results_writer = csv.writer(best_known_file, delimiter=',', quotechar='"', quoting=csv.QUOTE_MINIMAL)
        results_writer.writerow(["Problem", "Best known", "Solver result", "Comparison"])

        for problem_data in experiment_config.data.problems:
            instance_format = problem_data.format
            extra_args = problem_data.extraArgs
            print("processing {} files in '{}'..".format(instance_format, problem_data.name))

            problem_root = "{}/{}/{}".format(root_temp_dir, problem_data.name, instance_format)
            download_and_extract(problem_data.url, problem_root)

            for problem_instance in problem_data.files:
                instance_name = problem_instance.name
                instance_path = "{}/{}".format(problem_root, problem_instance.path)
                print("processing {}".format(instance_name))

                solution_path = "{}/{}_{}_solution_{}.txt".format(solver_cli.path, instance_format, instance_name,
                                                                  solver_config.name)

                if instance_format == 'pragmatic':
                    print("skip '{}' in pragmatic format".format(problem_data.name))
                    continue

                _, _, _, _, cost, _, _ = solver_cli.client.solve_scientific(instance_format, instance_path,
                                                                            solver_config_path, solution_path,
                                                                            extra_args)

                best_known_cost = get_best_known_cost(instance_path)

                if best_known_cost < cost:
                    percentage = 100 * (cost - best_known_cost) / best_known_cost
                    print("{}: WORSE solution: {} vs {} ({:.2f}%)".format(instance_name, cost, best_known_cost,
                                                                          percentage))
                    comparison = "worse"
                elif best_known_cost > cost:
                    print("{}: BETTER solution: {} vs {}".format(instance_name, cost, best_known_cost))
                    comparison = "better"
                else:
                    print("{}: SAME".format(instance_name))
                    comparison = "same"

                if comparison != "same":
                    shutil.copy(solution_path, results_output_path)

                results_writer.writerow([instance_name, best_known_cost, cost, comparison])
