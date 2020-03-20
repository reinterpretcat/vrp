# First steps

## CLI tool

The project provides command line tool which you can use to solve VRP without need to write any code. Depending on your
problem type and settings, you might need to specify different command line arguments, for example:

- solve scientific problem from **Li&Lim** set writing solution to the file specified

      vrp-cli lilim LC1_10_2.txt -o LC1_10_2_solution.txt

- solve scientific problem from **Solomon** set using existing solution

      vrp-cli solomon RC1_10_1.txt --init-solution RC1_10_1_solution.txt

- solve custom problem specified in **pragmatic** json format with its routing matrix

      vrp-cli pragmatic problem.json -m routing_matrix.json --max-generations=1000

For all options, simply run

     vrp-cli --help


## Programmatic usage

You can build and use project as a library as it exposes related functionality as `extern C` function. Please refer
to [interop examples page](../examples/interop/index.md).
