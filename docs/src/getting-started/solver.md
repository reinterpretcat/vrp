# Running solver

To run the solver, simply use:

    vrp-cli solve pragmatic problem.json -o solution.json -g solution.geojson


It will produce some log output which contains various information regarding refinement process such as costs, amount
of routes, time, etc.:

    configured to use single approximated routing matrix
    configured to use default max-generations (2000) and max-time (300secs)
    generation 1 took 6085ms (total 6s), cost: 4161.71 (100.000%), routes: 52, unassigned: 0, accepted: true
    generation 15 took 3ms (total 6s), cost: 4161.15 (-0.010%), routes: 52, unassigned: 0, accepted: true
    ....
    generation 996 took 10ms (total 34s), cost: 4081.04 (-0.006%), routes: 52, unassigned: 0, accepted: true
    generation 1000 took 5ms (total 34s), cost: 4084.25 (0.079%), routes: 52, unassigned: 0, accepted: false
           population state after 34s (speed: 28.82 gen/sec):
                   0 cost: 4081.04 (0.000%), routes: 52, unassigned: 0, discovered at: 996
                   1 cost: 4081.29 (0.006%), routes: 52, unassigned: 0, discovered at: 977
                   2 cost: 4082.12 (0.027%), routes: 52, unassigned: 0, discovered at: 969
                   3 cost: 4082.37 (0.033%), routes: 52, unassigned: 0, discovered at: 966
                   4 cost: 4082.43 (0.034%), routes: 52, unassigned: 0, discovered at: 947
    generation 1032 took 7ms (total 35s), cost: 4080.87 (-0.004%), routes: 52, unassigned: 0, accepted: true
    generation 1054 took 6ms (total 36s), cost: 4080.32 (-0.014%), routes: 52, unassigned: 0, accepted: true
    ...
    stopped due to termination (true) or goal satisfaction (false)
    solving took 64s, total generations: 2000, speed: 31.37 gen/sec
    best solution within cost 4051.2276 discovered at 1998 generation

Once the problem is solved, it will save solution in `pragmatic` and `geojson` (optional) format.

## Extra options

### Termination criteria

Termination criteria defines when refinement algorithm should stop and return best known solution. At the moment, there
are two types.

#### Max time

Max time specifies duration of solving in seconds:

    vrp-cli solve pragmatic problem.json -m routing_matrix.json -o solution.json --max-time=600

#### Max generations

Generation is one refinement step and it can be limited via _max-generations_ parameter:

    vrp-cli solve pragmatic problem.json -m routing_matrix.json -o solution.json --max-generations=1000


#### Default behavior

By default termination criteria is max 2000 generations or 300 seconds.


### Writing solution to file

Writing solution into file is controlled by `-o` or `--out-result` setting. When it is omitted, then solution is written
in std out.

Pragmatic format supports option `-g` or `--geo-json` which writes solution in separate file in geojson format.
