# Pragmatic

Pragmatic format aims to model a multiple VRP variants through single problem and solution model schemas which are
described in details in next sections.


## Performance

There is no limit on problem size, solver should be able to solve problems with thousands of jobs in fairly reasonable
amount of time depending on your termination criteria (e.g. time or amount of iterations/generations). However, exact
performance depends on VRP variant (e.g. _VRPPD_ is slower than _CVRPTW_).


## Quality of results

Although results seems to be comparable with alternative solutions, default metaheuristic still can be improved.


## Examples

A various examples can be found in [pragmatic examples section](../../examples/pragmatic/index.md).