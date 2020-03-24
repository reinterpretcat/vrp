# Command Line Tool

The project provides command line tool which is simplest way to solve VRP problem as you don't need to write any code.
It supports the following formats:

- **scientific**: a text format widely used for benchmarking various a algorithms in scientific papers. It has two types:
    - **solomon**: see [Solomon benchmark](https://www.sintef.no/projectweb/top/vrptw/solomon-benchmark)
    - **lilim**: see [Li&Lim benchmark](https://www.sintef.no/projectweb/top/pdptw/li-lim-benchmark)

- **pragmatic**: a custom json format designed to solve real world VRP problems.

Here some examples quick example:

- solve scientific problem from **Li&Lim** set writing solution to the file specified

      vrp-cli solve lilim LC1_10_2.txt -o LC1_10_2_solution.txt

- solve scientific problem from **Solomon** set using existing solution

      vrp-cli solve solomon RC1_10_1.txt --init-solution RC1_10_1_solution.txt

- solve custom problem specified in **pragmatic** json format with its routing matrix

      vrp-cli solve pragmatic problem.json -m routing_matrix.json --max-generations=1000

For all options, simply run

     vrp-cli --help


The next section provides an overview of available options.