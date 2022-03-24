# Running solver

To run the solver, simply use:

    vrp-cli solve pragmatic problem.json -o solution.json -g solution.geojson --log

If you specify `--log` option, it will produce some log output which contains various information regarding refinement
process such as costs, amount of routes, time, etc.:

```
configured to use max-time: 300s
configured to use custom heuristic
preparing initial solution(-s)
[0s] created initial solution in 536ms, fitness: (0.000, 104.000, 70928.735)
[1s] created initial solution in 513ms, fitness: (0.000, 109.000, 74500.133)
[2s] created initial solution in 1097ms, fitness: (0.000, 104.000, 70928.735)
[2s] created initial solution in 168ms, fitness: (0.000, 125.000, 94305.015)
created initial population in 2317ms
[2s] generation 0 took 21ms, fitness: (0.000, 104.000, 70669.056)
[2s] population state (phase: initial, speed: 0.00 gen/sec, improvement ratio: 1.000:1.000):
        rank: 0, fitness: (0.000, 104.000, 70669.056), improvement: 0.000%
        rank: 1, fitness: (0.000, 104.000, 70705.550), improvement: 0.052%
[5s] generation 100 took 27ms, fitness: (0.000, 96.000, 64007.851)
[7s] generation 200 took 19ms, fitness: (0.000, 95.000, 63087.282)
..
149s] generation 4000 took 44ms, fitness: (0.000, 92.000, 54032.930)
[149s] population state (phase: exploration, speed: 26.78 gen/sec, improvement ratio: 0.235:0.155):
        rank: 0, fitness: (0.000, 92.000, 54032.930), improvement: 0.000%
        rank: 1, fitness: (0.000, 92.000, 54032.930), improvement: 0.000%
[153s] generation 4100 took 42ms, fitness: (0.000, 92.000, 54021.021)
..
[297s] generation 7200 took 20ms, fitness: (0.000, 92.000, 53264.644)
[299s] population state (phase: exploitation, speed: 24.16 gen/sec, improvement ratio: 0.165:0.058):
        rank: 0, fitness: (0.000, 92.000, 53264.026), improvement: 0.000%
        rank: 1, fitness: (0.000, 92.000, 53264.026), improvement: 0.000%
[299s] total generations: 7246, speed: 24.16 gen/sec
Route 1: 144 925 689 739 358 32 783 924 461 111 766 842 433
..
Route 92: 837 539 628 847 740 585 328 666 785 745
Cost 53264.03
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
