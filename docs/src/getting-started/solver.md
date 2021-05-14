# Running solver

To run the solver, simply use:

    vrp-cli solve pragmatic problem.json -o solution.json -g solution.geojson --log

If you specify `--log` option, it will produce some log output which contains various information regarding refinement
process such as costs, amount of routes, time, etc.:

```
provided 0 initial solutions to start with, max init size: default
configured to use max-time: 300s
problem has total jobs: 1000, actors: 250
preparing initial solution(-s)
[0s] created 1 of 7 initial solutions in 582ms (ts: 0.0019427497533333333)
[1s] created 2 of 7 initial solutions in 635ms (ts: 0.0040622654700000005)
[2s] created 3 of 7 initial solutions in 1198ms (ts: 0.00805636654)
[3s] created 4 of 7 initial solutions in 1018ms (ts: 0.011452720063333333)
[4s] created 5 of 7 initial solutions in 1138ms (ts: 0.01524855115)
[5s] created 6 of 7 initial solutions in 377ms (ts: 0.016506068766666666)
[5s] created 7 of 7 initial solutions in 618ms (ts: 0.018567976263333334)
[5s] generation 0 took 5570ms, rank: 0, cost: 70410.91(0.000%), tours: 102, unassigned: 0, fitness: (0.000, 102.000, 70410.908)
[5s] population state (phase: initial, speed: 0.00 gen/sec, improvement ratio: 1.000:1.000):
         rank: 0, cost: 70410.91(0.000%), tours: 102, unassigned: 0, fitness: (0.000, 102.000, 70410.908)
         rank: 1, cost: 70928.74(0.735%), tours: 104, unassigned: 0, fitness: (0.000, 104.000, 70928.735)
[9s] generation 100 took 35ms, rank: 0, cost: 63979.25(0.000%), tours: 96, unassigned: 0, fitness: (0.000, 96.000, 63979.253)
[13s] generation 200 took 41ms, rank: 0, cost: 60138.98(0.000%), tours: 96, unassigned: 0, fitness: (0.000, 96.000, 60138.981)
...
[178s] generation 5000 took 24ms, rank: 0, cost: 54446.61(0.000%), tours: 92, unassigned: 0, fitness: (0.000, 92.000, 54446.606)
[178s] population state (phase: exploration, speed: 28.08 gen/sec, improvement ratio: 0.246:0.070):
         rank: 0, cost: 54446.61(0.000%), tours: 92, unassigned: 0, fitness: (0.000, 92.000, 54446.606)
         rank: 1, cost: 54446.61(0.000%), tours: 92, unassigned: 0, fitness: (0.000, 92.000, 54446.606)
[181s] generation 5100 took 30ms, rank: 0, cost: 54408.79(0.000%), tours: 92, unassigned: 0, fitness: (0.000, 92.000, 54408.791)
...
[294s] population state (phase: exploitation, speed: 30.53 gen/sec, improvement ratio: 0.196:0.173):
         rank: 0, cost: 54180.15(0.000%), tours: 91, unassigned: 0, fitness: (0.000, 91.000, 54180.148)
         rank: 1, cost: 54180.15(0.000%), tours: 91, unassigned: 0, fitness: (0.000, 91.000, 54180.148)
[296s] generation 9100 took 12ms, rank: 0, cost: 54164.63(0.000%), tours: 91, unassigned: 0, fitness: (0.000, 91.000, 54164.626)
[298s] generation 9200 took 16ms, rank: 0, cost: 54160.28(0.000%), tours: 91, unassigned: 0, fitness: (0.000, 91.000, 54160.282)
[300s] population state (phase: exploitation, speed: 30.89 gen/sec, improvement ratio: 0.192:0.117):
         rank: 0, cost: 54160.28(0.000%), tours: 91, unassigned: 0, fitness: (0.000, 91.000, 54160.282)
         rank: 1, cost: 54160.28(0.000%), tours: 91, unassigned: 0, fitness: (0.000, 91.000, 54160.282)
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


#### Coefficient of variation

This criteria calculates [coefficient of variation](https://en.wikipedia.org/wiki/Coefficient_of_variation) for each
objective over specific amount of generations specified by `sample` and stops algorithm when all calculated values are
below specified `threshold`. It can be defined by `min-cv` parameter:

    vrp-cli solve pragmatic problem.json --min-cv=sample,200,0.1,true

Here, the first parameter is generation amount, the second - threshold, the third - boolean flag whether the logic is
applicable for all search phases, not only exploitation.

Alternatively, period of time in seconds can be specified instead of sample:

    vrp-cli solve pragmatic problem.json --min-cv=period,300,0.1,true

Due to internal search heuristic implementation, it is recommended to use this termination criteria with `max-time` or
`max-generations`.

#### Default behavior

Default termination criteria is max 3000 generations and 300 seconds at max.


### Initial solution

You can supply initial solution to start with using `-i` option. Amount of initial solutions to be built can be
overridden using `init-size` option.


### Writing solution to file

Writing solution into file is controlled by `-o` or `--out-result` setting. When it is omitted, then solution is written
in std out.

Pragmatic format supports option `-g` or `--geo-json` which writes solution in separate file in geojson format.
