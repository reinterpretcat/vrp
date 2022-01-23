# Running solver

To run the solver, simply use:

    vrp-cli solve pragmatic problem.json -o solution.json -g solution.geojson --log

If you specify `--log` option, it will produce some log output which contains various information regarding refinement
process such as costs, amount of routes, time, etc.:

```
configured to use max-time: 300s
preparing initial solution(-s)
[0s] created 1 of 4 initial solutions in 502ms
        fitness: (0.000, 104.000, 70928.735)
[1s] created 2 of 4 initial solutions in 510ms
        fitness: (0.000, 109.000, 74500.133)
[2s] created 3 of 4 initial solutions in 1032ms
        fitness: (0.000, 104.000, 70928.735)
[2s] created 4 of 4 initial solutions in 166ms
        fitness: (0.000, 127.000, 99956.273)
[2s] generation 0 took 2214ms, rank: 0, fitness: (0.000, 104.000, 70928.735), improvement: 0.000%
[2s] population state (phase: initial, speed: 0.00 gen/sec, improvement ratio: 1.000:1.000):
         rank: 0, fitness: (0.000, 104.000, 70928.735), improvement: 0.000%
[5s] generation 100 took 15ms, rank: 0, fitness: (0.000, 97.000, 63075.095), improvement: 0.000%
..
[38s] generation 900 took 19ms, rank: 0, fitness: (0.000, 94.000, 55739.089), improvement: 0.000%
[43s] generation 1000 took 113ms, rank: 0, fitness: (0.000, 94.000, 55356.272), improvement: 0.000%
[43s] population state (phase: exploration, speed: 23.13 gen/sec, improvement ratio: 0.386:0.385):
         rank: 0, fitness: (0.000, 94.000, 55356.272), improvement: 0.000%
         rank: 1, fitness: (0.000, 94.000, 55376.441), improvement: 0.036%
..
[227s] generation 5300 took 28ms, rank: 0, fitness: (0.000, 92.000, 53623.011), improvement: 0.000%
[232s] generation 5400 took 25ms, rank: 0, fitness: (0.000, 92.000, 53323.363), improvement: 0.000%
[236s] generation 5500 took 33ms, rank: 0, fitness: (0.000, 91.000, 55788.493), improvement: 0.000%
..
[300s] population state (phase: exploitation, speed: 23.47 gen/sec, improvement ratio: 0.193:0.179):
         rank: 0, fitness: (0.000, 91.000, 54009.256), improvement: 0.000%
         rank: 1, fitness: (0.000, 91.000, 54009.256), improvement: 0.000%
[300s] total generations: 7042, speed: 23.47 gen/sec
Route 1: 179 892 828 821 431 211 55 512 473 95 897 758
Route 2: 798 962 999 773 443 3 949 213 499 848 392 72
..
Route 91: 844 364 377 452 165 41 336 380 172 403
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
