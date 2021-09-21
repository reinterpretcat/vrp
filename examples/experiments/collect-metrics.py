import csv
import os
import sys
import tempfile
from shared import download_and_extract, prepare_environment

if len(sys.argv) < 2:
    print("Provide url to experiment config")
    sys.exit(1)
else:
    experiment_config_url = sys.argv[1]

with tempfile.TemporaryDirectory() as root_temp_dir:
    solver_config_root, experiment_config, solver_versions = prepare_environment(root_temp_dir, experiment_config_url)
    experiment_output_path = "{}/experiments-results.csv".format(os.path.dirname(os.path.realpath(__file__)))

    with open(experiment_output_path, mode='w') as best_known_file:
        best_known_writer = csv.writer(best_known_file, delimiter=',', quotechar='"', quoting=csv.QUOTE_MINIMAL)
        best_known_writer.writerow(
            ["Version", "Config", "Problem", "Type", "Iteration", "Duration", "Generations", "Speed", "Cost", "Tours",
             "Unassigned"])

        for solver_config in experiment_config.data.config.files:
            print("solver config: {}".format(solver_config.name))
            solver_config_path = "{}/{}".format(solver_config_root, solver_config.path)

            for problem_data in experiment_config.data.problems:
                instance_format = problem_data.format
                print("processing {} files in '{}'..".format(instance_format, problem_data.name))

                problem_root = "{}/{}/{}".format(root_temp_dir, problem_data.name, instance_format)
                download_and_extract(problem_data.url, problem_root)

                for iteration_number in range(experiment_config.parameters.iterations):
                    print("iteration {}".format(iteration_number))

                    for problem_instance in problem_data.files:
                        instance_name = problem_instance.name
                        instance_path = "{}/{}".format(problem_root, problem_instance.path)
                        print("processing {}".format(instance_name))

                        for solver_cli in solver_versions:
                            print("solver version name: {}".format(solver_cli.name))
                            solution_path = "{}/{}_{}_solution_{}_{}.json".format(
                                solver_cli.path, instance_format, instance_name, solver_config.name, iteration_number)

                            if instance_format == 'pragmatic':
                                instance_matrices = problem_instance.matrices
                                solution = solver_cli.client.solve_pragmatic(
                                    instance_path, instance_matrices, solver_config_path, solution_path)

                                best_known_writer.writerow([
                                    solver_cli.name, solver_config.name, instance_name, 'pragmatic', iteration_number,
                                    solution.extras.metrics.duration,
                                    solution.extras.metrics.generations,
                                    "{:.4f}".format(round(solution.extras.metrics.speed, 2)),
                                    "{:.4f}".format(round(solution.statistic.cost, 2)),
                                    len(solution.tours), len(getattr(solution, "unassigned", []))
                                ])
                            else:
                                _, duration, generations, speed, cost, tours, unassigned = \
                                    solver_cli.client.solve_scientific(instance_format, instance_path,
                                                                       solver_config_path, solution_path)

                                best_known_writer.writerow([
                                    solver_cli.name, solver_config.name, instance_name, instance_format,
                                    iteration_number, duration, generations, speed, cost, tours, unassigned
                                ])
