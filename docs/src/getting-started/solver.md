# Running solver

To run the solver, simply use:

    vrp-cli solve pragmatic problem.json -o solution.json -g solution.geojson --log

If you specify `--log` option, it will produce some log output which contains various information regarding refinement
process such as costs, amount of routes, time, etc.:

```
provided 0 initial solutions to start with, max init size: default
configured to use max-time: 120s
problem has total jobs: 1000, actors: 250
preparing initial solution(-s)
[0s] created 1 of 4 initial solutions in 622ms (ts: 0.005186183741666667)
        cost: 70928.74, tours: 104, unassigned: 0, fitness: (0.000, 104.000, 70928.735)
[1s] created 2 of 4 initial solutions in 655ms (ts: 0.010658395816666666)
        cost: 74667.55, tours: 109, unassigned: 0, fitness: (0.000, 109.000, 74667.552)
[2s] created 3 of 4 initial solutions in 1356ms (ts: 0.021964635916666666)
        cost: 92749.84, tours: 147, unassigned: 0, fitness: (0.000, 147.000, 92749.835)
[3s] created 4 of 4 initial solutions in 237ms (ts: 0.02394588325)
        cost: 104125.42, tours: 132, unassigned: 0, fitness: (0.000, 132.000, 104125.418)
[3s] generation 0 took 2873ms, rank: 0, cost: 70928.74(0.000%), tours: 104, unassigned: 0, fitness: (0.000, 104.000, 70928.735)
[3s] population state (phase: initial, speed: 0.00 gen/sec, improvement ratio: 1.000:1.000):
         rank: 0, cost: 70928.74(0.000%), tours: 104, unassigned: 0, fitness: (0.000, 104.000, 70928.735)
[9s] generation 100 took 56ms, rank: 0, cost: 61572.58(0.000%), tours: 97, unassigned: 0, fitness: (0.000, 97.000, 61572.584)
[13s] generation 200 took 32ms, rank: 0, cost: 60189.31(0.000%), tours: 95, unassigned: 0, fitness: (0.000, 95.000, 60189.313)
..
[42s] generation 900 took 45ms, rank: 0, cost: 57367.53(0.000%), tours: 93, unassigned: 0, fitness: (0.000, 93.000, 57367.531)
[46s] generation 1000 took 33ms, rank: 0, cost: 57051.52(0.000%), tours: 93, unassigned: 0, fitness: (0.000, 93.000, 57051.516)
[46s] population state (phase: exploration, speed: 21.58 gen/sec, improvement ratio: 0.444:0.443):
         rank: 0, cost: 57051.52(0.000%), tours: 93, unassigned: 0, fitness: (0.000, 93.000, 57051.516)
         rank: 1, cost: 57177.18(0.220%), tours: 93, unassigned: 0, fitness: (0.000, 93.000, 57177.178)
..
[115s] generation 2800 took 30ms, rank: 0, cost: 54100.07(0.000%), tours: 92, unassigned: 0, fitness: (0.000, 92.000, 54100.067)
[118s] generation 2900 took 29ms, rank: 0, cost: 54041.48(0.000%), tours: 92, unassigned: 0, fitness: (0.000, 92.000, 54041.477)
[120s] population state (phase: exploitation, speed: 24.49 gen/sec, improvement ratio: 0.293:0.165):
         rank: 0, cost: 54041.48(0.000%), tours: 92, unassigned: 0, fitness: (0.000, 92.000, 54041.477)
         rank: 1, cost: 54041.48(0.000%), tours: 92, unassigned: 0, fitness: (0.000, 92.000, 54041.477)
[120s] total generations: 2945, speed: 24.49 gen/sec
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
