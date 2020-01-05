# Command Line Tool

Depending on your problem type and settings, you might need to specify different command line
arguments, for example:

- solve scientific problem from **solomon** set using existing solution

        vrp-cli solomon RC1_10_1.txt --init-solution RC1_10_1_solution.txt  --max-time=3600

- solve custom problem specified in **pragmatic** json format with its routing matrix.

       vrp-cli pragmatic problem_definition.json -m routing_matrix.json --max-generations=1000`

- solve scientific problem from **li lim** set writing solution to the file specified

        vrp-cli lilim LC1_10_2.txt -o LC1_10_2_solution.txt

For more details, simply run

     vrp-cli --help