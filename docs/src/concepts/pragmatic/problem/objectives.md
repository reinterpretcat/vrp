# Objectives

A classical objective function (or simply objective) for VRP is minimization of total cost. However, real life scenarios
require different objective function or even more than one considered simultaneously. That's why the solver has a concept
of multi objective.


## Understanding multi objective structure

A multi objective is defined by `objectives` and consists of two properties:

- **primary** (required): a list of primary objectives, at least one must be present
- **secondary** (optional): a list of secondary objectives

Splitting multiple objectives into two separate collections serves the purpose to solve the problem that many objectives
are conflicting by their nature. So, secondary objectives are considered only if objectives in primary list cannot detect
the change in newly discovered solution.


## Available objectives

The solver already provides multiple built-in objectives distinguished by their `type`. All these objectives can be
split into two groups.

### Scalar objectives

This objectives targeting for some scalar characteristic of solution:

* `minimize-cost`: minimizes total transport cost calculated for all routes
* `minimize-unassigned` objective minimizes amount of unassigned jobs. Although, solver tries to minimize amount of
unassigned jobs all the time, it is possible that solution, discovered during refinement, has more unassigned jobs than
previously accepted. The reason of that can be conflicting objective (e.g. fleet minimization) and restrictive
constraints such as time windows
* `minimize-tours`: minimizes total amount of tours present in solution
* `maximize-tours`: maximizes total amount of tours present in solution

### Work balance objectives

There are four work balance objectives available:

* `balance-max-load`: balances max load in tour
* `balance-activites`: balances amount of activities performed in tour
* `balance-distance`: balances travelled distance per tour
* `balance-duration`: balances tour durations

Each objective has optional parameters defined by `option` property:
* `threshold`: a relative value in single tour before balancing takes place, it is soft constraint and might be
 ignored by decision maker
* `tolerance`: a tolerance by variation coefficient

An usage example:

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/multi-objective.balance-load.problem.json:151:157}}
```

## Default behaviour

By default, decision maker minimizes amount of routes, unassigned jobs and total cost which is equal to the following
definition:

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/multi-objective.default.problem.json:138:152}}
```

Here, cost minimization is a secondary objective which corresponds to a classical hierarchical objective used
by `Solomon` benchmark.


## Related errors

* [E1600 an empty objective specified](../errors/index.md#e1600)
* [E1601 duplicate objective specified](../errors/index.md#e1601)
* [E1602 missing cost objective](../errors/index.md#e1602)


## Examples

Please refer to [examples section](../../../examples/pragmatic/objectives/index.md) to see more examples.
