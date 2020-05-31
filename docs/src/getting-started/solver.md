# Running solver

To run the solver, simply use:

    vrp-cli solve pragmatic problem.json -o solution.json -g solution.geojson --log

If you specify `--log` option, it will produce some log output which contains various information regarding refinement
process such as costs, amount of routes, time, etc.:

```
configured to use single approximated routing matrix
provided 0 initial solutions to start with
configured to use max-generations 1000
[0s] created 1 of 2 initial solutions in 2ms
[0s] created 2 of 2 initial solutions in 0ms
[0s] population state (speed: 272.68 gen/sec):
        cost: 114.29 (0.000%), tours: 2, unassigned: 0
        cost: 117.58 (2.876%), tours: 2, unassigned: 1
[0s] generation 100 took 0ms, cost: 69.70, tours: 2, unassigned: 0
..
[0s] generation 800 took 0ms, cost: 69.70, tours: 2, unassigned: 0
[0s] generation 900 took 0ms, cost: 69.70, tours: 2, unassigned: 0
[0s] population state (speed: 7498.72 gen/sec):
        cost: 69.70 (0.000%), tours: 2, unassigned: 0
        cost: 69.47 (-0.319%), tours: 2, unassigned: 0
        cost: 69.76 (0.089%), tours: 2, unassigned: 0
        cost: 69.54 (-0.230%), tours: 2, unassigned: 0
        cost: 71.76 (2.966%), tours: 2, unassigned: 0
        cost: 113.12 (62.307%), tours: 2, unassigned: 0
        cost: 114.29 (63.988%), tours: 2, unassigned: 0
[0s] total generations: 1000, speed: 7495.83 gen/sec
best solution has cost: 69.6964, tours: 2, unassigned: 0

```
Once the problem is solved, it will save solution in `pragmatic` and `geojson` (optional) format.

## Extra options

### Termination criteria

Termination criteria defines when refinement algorithm should stop and return best known solution. At the moment, there
are two types.

#### Max time

Max time specifies duration of solving in seconds:

    vrp-cli solve pragmatic problem.json --max-time=600

#### Max generations

Generation is one refinement step and it can be limited via _max-generations_ parameter:

    vrp-cli solve pragmatic problem.json --max-generations=1000

#### Cost variation

Cost variation stops refinement process when cost does not significantly change:

    vrp-cli solve pragmatic problem.json --cost-variation=200,0.1

It calculates [coefficient of variation](https://en.wikipedia.org/wiki/Coefficient_of_variation) of cost change over
specific amount of generations specified by `sample` and stops algorithm when it is below specified `threshold`.


#### Default behavior

By default termination criteria is max 2000 generations or 300 seconds.


### Writing solution to file

Writing solution into file is controlled by `-o` or `--out-result` setting. When it is omitted, then solution is written
in std out.

Pragmatic format supports option `-g` or `--geo-json` which writes solution in separate file in geojson format.
