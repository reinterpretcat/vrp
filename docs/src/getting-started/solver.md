# Running solver

To run the solver, simply use:

    vrp-cli solve pragmatic problem.json -o solution.json -g solution.geojson --log

If you specify `--log` option, it will produce some log output which contains various information regarding refinement
process such as costs, amount of routes, time, etc.:

```
configured to use max-time: 30s
problem has total jobs: 1000, actors: 250
[0s] created 1 of 1 initial solutions in 487ms
[0s] generation 0 took 487ms, rank: 0, cost: 70928.74(0.000%), tours: 104, unassigned: 0, fitness: (0.000, 104.000, 70928.735)
[0s] population state (speed: 0.00 gen/sec, improvement ratio: 1.000:1.000):
         rank: 0, cost: 70928.74(0.000%), tours: 104, unassigned: 0, fitness: (0.000, 104.000, 70928.735)
[2s] generation 100 took 17ms, rank: 0, cost: 61115.24(0.000%), tours: 99, unassigned: 0, fitness: (0.000, 99.000, 61115.241)
[5s] generation 200 took 35ms, rank: 0, cost: 57736.43(0.000%), tours: 98, unassigned: 0, fitness: (0.000, 98.000, 57736.427)
[7s] generation 300 took 20ms, rank: 0, cost: 56682.93(0.000%), tours: 97, unassigned: 0, fitness: (0.000, 97.000, 56682.934)

[24s] population state (speed: 41.18 gen/sec, improvement ratio: 0.362:0.361):
         rank: 0, cost: 54495.90(0.000%), tours: 96, unassigned: 0, fitness: (0.000, 96.000, 54495.896)
         rank: 1, cost: 54501.45(0.010%), tours: 96, unassigned: 0, fitness: (0.000, 96.000, 54501.454)
[26s] generation 1100 took 22ms, rank: 0, cost: 54364.42(0.000%), tours: 96, unassigned: 0, fitness: (0.000, 96.000, 54364.415)
[28s] generation 1200 took 24ms, rank: 0, cost: 54204.55(0.000%), tours: 96, unassigned: 0, fitness: (0.000, 96.000, 54204.548)
[30s] population state (speed: 41.29 gen/sec, improvement ratio: 0.319:0.231):
         rank: 0, cost: 54204.55(0.000%), tours: 96, unassigned: 0, fitness: (0.000, 96.000, 54204.548)
         rank: 1, cost: 54204.97(0.001%), tours: 96, unassigned: 0, fitness: (0.000, 96.000, 54204.965)
[30s] total generations: 1239, speed: 41.29 gen/sec
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

#### Default behavior

By default termination criteria is max 3000 generations and 300 seconds at max.

### Initial solution

You can supply initial solution to start with using `-i` option.

### Writing solution to file

Writing solution into file is controlled by `-o` or `--out-result` setting. When it is omitted, then solution is written
in std out.

Pragmatic format supports option `-g` or `--geo-json` which writes solution in separate file in geojson format.
