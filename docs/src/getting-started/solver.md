# Running solver

To run the solver, simply use:

    vrp-cli solve pragmatic problem.json -o solution.json -g solution.geojson --log

If you specify `--log` option, it will produce some log output which contains various information regarding refinement
process such as costs, amount of routes, time, etc.:

```
configured to use single approximated routing matrix
provided 0 initial solutions to start with
configured to use max-time: 5s
[0s] created 1 of 1 initial solutions in 5ms
[0s] population state (speed: 123.55 gen/sec, improvement ratio: 1.000:1.000):
         rank: 0, cost: 518.91(0.000%), tours: 10, unassigned: 0, fitness: (0.000, 10.000, 518.913)
         rank: 1, cost: 520.10(0.229%), tours: 10, unassigned: 0, fitness: (0.000, 10.000, 520.103)
         rank: 2, cost: 522.22(0.637%), tours: 10, unassigned: 0, fitness: (0.000, 10.000, 522.221)
         rank: 3, cost: 522.54(0.699%), tours: 10, unassigned: 0, fitness: (0.000, 10.000, 522.541)
[0s] generation 100 took 2ms,  rank: 0, cost: 512.84(0.000%), tours: 10, unassigned: 0, fitness: (0.000, 10.000, 512.836)
..
[4s] generation 1900 took 2ms,  rank: 0, cost: 506.64(0.000%), tours: 10, unassigned: 0, fitness: (0.000, 10.000, 506.644)
[5s] population state (speed: 393.37 gen/sec, improvement ratio: 0.032:0.005):
         rank: 0, cost: 506.46(0.000%), tours: 10, unassigned: 0, fitness: (0.000, 10.000, 506.458)
         rank: 1, cost: 506.48(0.005%), tours: 10, unassigned: 0, fitness: (0.000, 10.000, 506.484)
         rank: 2, cost: 506.49(0.007%), tours: 10, unassigned: 0, fitness: (0.000, 10.000, 506.494)
         rank: 3, cost: 506.51(0.010%), tours: 10, unassigned: 0, fitness: (0.000, 10.000, 506.509)
[5s] total generations: 1967, speed: 393.37 gen/sec
```
Once the problem is solved, it will save solution in `pragmatic` and `geojson` (optional) format.

## Extra options

### Termination criteria

Termination criteria defines when refinement algorithm should stop and return best known solution. At the moment, there
are three types which can be used simultaneously:

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

### Initial solution

You can supply initial solution to start with using `-i` option.

#### Default behavior

By default termination criteria is max 3000 generations and 300 seconds at max.


### Writing solution to file

Writing solution into file is controlled by `-o` or `--out-result` setting. When it is omitted, then solution is written
in std out.

Pragmatic format supports option `-g` or `--geo-json` which writes solution in separate file in geojson format.
