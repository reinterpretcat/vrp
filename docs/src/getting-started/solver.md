# Running solver

To run the solver, simply use:

    vrp-cli solve pragmatic problem.json -o solution.json -g solution.geojson --log

If you specify `--log` option, it will produce some log output which contains various information regarding refinement
process such as costs, amount of routes, time, etc.:

```
provided 0 initial solutions to start with
configured to use default max-generations (3000) and max-time (300secs)
problem has total jobs: 1000, actors: 250
[0s] created 1 of 1 initial solutions in 471ms
[0s] generation 0 took 472ms, rank: 0, cost: 70928.74(0.000%), tours: 104, unassigned: 0, fitness: (0.000, 104.000, 70928.735)
[0s] population state (phase: initial, speed: 0.00 gen/sec, improvement ratio: 1.000:1.000):
         rank: 0, cost: 70928.74(0.000%), tours: 104, unassigned: 0, fitness: (0.000, 104.000, 70928.735)
[3s] generation 100 took 23ms, rank: 0, cost: 65448.21(0.000%), tours: 101, unassigned: 0, fitness: (0.000, 101.000, 65448.214)
[5s] generation 200 took 22ms, rank: 0, cost: 63573.34(0.000%), tours: 101, unassigned: 0, fitness: (0.000, 101.000, 63573.343)
...
[24s] generation 900 took 21ms, rank: 0, cost: 58896.38(0.000%), tours: 98, unassigned: 0, fitness: (0.000, 98.000, 58896.376)
[27s] generation 1000 took 25ms, rank: 0, cost: 58698.43(0.000%), tours: 98, unassigned: 0, fitness: (0.000, 98.000, 58698.427)
[27s] population state (phase: exploration, speed: 36.57 gen/sec, improvement ratio: 0.244:0.243):
         rank: 0, cost: 58698.43(0.000%), tours: 98, unassigned: 0, fitness: (0.000, 98.000, 58698.427)
         rank: 1, cost: 58698.76(0.001%), tours: 98, unassigned: 0, fitness: (0.000, 98.000, 58698.755)
[30s] generation 1100 took 27ms, rank: 0, cost: 58118.80(0.000%), tours: 98, unassigned: 0, fitness: (0.000, 98.000, 58118.802)
...
[82s] population state (phase: exploitation, speed: 36.48 gen/sec, improvement ratio: 0.178:0.173):
         rank: 0, cost: 55847.51(0.000%), tours: 94, unassigned: 0, fitness: (0.000, 94.000, 55847.507)
         rank: 1, cost: 55848.46(0.002%), tours: 94, unassigned: 0, fitness: (0.000, 94.000, 55848.457)
[82s] total generations: 3000, speed: 36.48 gen/sec
```
Once the problem is solved, it will save solution in `pragmatic` and `geojson` (optional) format.

## Extra options

The `vrp-cli` supports extra command line arguments which affects behavior of the algorithm.


### Search mode

By default, search is mostly performed in exploration mode (`broad`) which allows the algorithm to perform better
exploration of a solution space. However, it has slower overall convergence in better local optimum.
You can switch to exploitation mode with `deep` setting:

    vrp-cli solve pragmatic problem.json --search-mode=deep

In this mode, the algorithm memorizes only the last discovered best known solutions, so it can jump quicker to relatively
good local optimum, but suffers more from premature convergence.

A general recommendation is to use `deep` on relatively simple dataset and/or when strict time limits should be applied.


### Heuristic mode

At the moment, the solver supports three types of hyper-heuristics:

* `static selective`: chooses metaheuristic from the list of predefined within their probabilities
* `dynamic selective`: applies reinforcement learning technics to adjust probabilities of predefined metaheuristics
* `multi selective` (default): starts with dynamic selective and switches to static selective if the progression speed is slow

You can switch between modes with `heuristic` setting:

    vrp-cli solve pragmatic problem.json --heuristic=static


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

Default termination criteria is max 3000 generations and 300 seconds at max.


### Initial solution

You can supply initial solution to start with using `-i` option. Amount of initial solutions to be built can be
overridden using `init-size` option.


### Writing solution to file

Writing solution into file is controlled by `-o` or `--out-result` setting. When it is omitted, then solution is written
in std out.

Pragmatic format supports option `-g` or `--geo-json` which writes solution in separate file in geojson format.
