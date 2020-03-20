# Logging information

CLI tool logs to std out some information about refinement progress:

    configured to use single approximated routing matrix
    configured to use default max-generations (2000) and max-time (300secs)
    generation 1 took 6085ms (total 6s), cost: 4161.71 (100.000%), routes: 52, unassigned: 0, accepted: true
    generation 14 took 3ms (total 6s), cost: 4161.58 (-0.003%), routes: 52, unassigned: 0, accepted: true
    generation 15 took 3ms (total 6s), cost: 4161.15 (-0.010%), routes: 52, unassigned: 0, accepted: true
    ....
    generation 977 took 5ms (total 33s), cost: 4081.29 (-0.020%), routes: 52, unassigned: 0, accepted: true
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

This log contains information about the costs, amount of routes, time, etc.
