# Solomon problems

To run the problem from `solomon` set, simply specify _solomon_ as a type. The following command solves solomon problem
defined in _RC1_10_1.txt_ and stores solution in _RC1_10_1_solution.txt_:

    vrp-cli solve solomon RC1_10_1.txt -o RC1_10_1_solution.txt

Optionally, you can specify initial solution to start with:

    vrp-cli solve solomon RC1_10_1.txt --init-solution RC1_10_1_solution_initial.txt -o RC1_10_1_solution_improved.txt


For details see [Solomon benchmark](https://www.sintef.no/projectweb/top/vrptw/solomon-benchmark).