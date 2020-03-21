# Objectives

A classical objective function (or simply objective) for VRP is minimization of total cost. However, real life scenarios
require different objective function or even more than one considered simultaneously. That's why the solver has a concept
of multi objective.

## Understanding multi objective structure

A multi objective is defined by `objectives` and consists of two properties:

- **primary** (required): a list of primary objectives, at least one must be present
- **secondary** (optional): a list of secondary objectives

Spliting multiple objectives into two separate collections serves the purpose to solve the problem that many objectives
are conflicting by their nature. So, secondary objectives are considered only if objectives in primary list cannot detect
the change in newly discovered solution.

All objectives in each list conforms `Pareto dominance` principle.

## Objective structure

There are multiple objective types distinguished by `type` property. Depending on type, an objective can have the
following characteristics:

- __goal__
- __threshold__

### Objectives with goal

For value-based objectives, it is possible to specify an optimization goal:

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/multi-objective.goal.problem.json:150:156}}
```

Goal consists of two optional properties:

- **value**: a desired value specific for each objective
- **variation**: a variation coefficient parameters to track objective value change

A goal considered as reached, if any of these parameters are reached. Once the goal is reached for all objectives in list,
refinement algorithm stops.


### Objectives with threshold

For balancing objective, an optional threshold specifies a value when it has to be applied:

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/multi-objective.balance-load.problem.json:140:143}}
```


## Available objectives

The solver already provides some built-in objectives.


### Total cost minimization

An objective with `minimize-cost` type minimizes total transport cost calculated for all routes:

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/multi-objective.goal.problem.json:148:157}}
```

The objective has `goal` with `value` and `variation` specifying desired total cost and variation coefficient.


### Fleet usage minimization

An objective with `minimize-tours` type minimizes total amount of tours present in solution:

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/multi-objective.goal.problem.json:140:142}}
```

The objective has `goal` with `value` and `variation` specifying desired tour amount and variation coefficient.


### Unassigned jobs minimization

A `minimize-unassigned` objective minimizes amount of unassigned jobs. Although, solver tries to minimize amount of
unassigned jobs all the time, it is possible that solution, discovered during refinement, has more unassigned jobs than
previously accepted. The reason of that can be conflicting objective (e.g. fleet minimization) and restrictive
constraints such as time windows.

In order to use this objective specify `minimize-unassigned` type:

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/multi-objective.goal.problem.json:143:145}}
```

The objective has `goal` with `value` and `variation` specifying desired amount of unassigned and variation coefficient.


### Work balance objectives

There are four work balance objectives available:

* `balance-max-load`: balances max load in tour
* `balance-activites`: balances amount of activities performed in tour
* `balance-distance`: balances travelled distance per tour
* `balance-duration`: balances tour durations


## Default behaviour

By default, decision maker minimizes amount of routes, unassigned jobs and total cost which is equal to the following
definition:

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/multi-objective.default.problem.json:138:152}}
```

Here, cost minimization is a secondary objective which corresponds to a classical hierarchical objective used
by `Solomon` benchmark.


## Notes

Most of objectives are conflicting to each other, so there are some suggestions:

* do not put cost minimization as primary objective within other minimization objectives. For example, newly discovered
  solution with less unassigned jobs is used to be more expensive than previously accepted. As result, this reduces
  the chance to minimize tours and, in most cases, the final total cost.
* goals, defined on secondary objectives, are ignored if there is at least one goal on primary objective.
* use common sense when mixing multiple objectives.


## Related errors

* [E1009 An empty objective specified](../errors/index.md#e1009)
* [E1010 Duplicate objective specified](../errors/index.md#e1010)
* [E1011 Missing cost objective](../errors/index.md#e1011)


## Examples

Please refer to [examples section](../../../examples/pragmatic/objectives/index.md) to see more.


