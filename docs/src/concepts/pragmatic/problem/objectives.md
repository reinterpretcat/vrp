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
* `minimize-unassigned`: minimizes amount of unassigned jobs. Although, solver tries to minimize amount of
unassigned jobs all the time, it is possible that solution, discovered during refinement, has more unassigned jobs than
previously accepted. The reason of that can be conflicting objective (e.g. minimize tours) and restrictive
constraints such as time windows. The objective has the following optional parameter:
    * `breaks`: a multiplicative coefficient to make breaks more preferable for assignment. Default value is 1. Setting
     this parameter to a value bigger than 1 is useful when it is highly desirable to have break assigned but its
     assignment leads to more jobs unassigned.
* `minimize-tours`: minimizes total amount of tours present in solution
* `maximize-tours`: maximizes total amount of tours present in solution

### Work balance objectives

There are four work balance objectives available:

* `balance-max-load`: balances max load in tour
* `balance-activites`: balances amount of activities performed in tour
* `balance-distance`: balances travelled distance per tour
* `balance-duration`: balances tour durations

Each objective has optional parameters defined by `option` property:
* `threshold`: a target coefficient of variation value which specifies desired minimum balancing level. All values below
threshold are considered equal which helps the search algorithm to optimize conflicting objectives.
* `tolerance`: a step tolerance by variation coefficient. Algorithm considers two fitness values equal if they differ
not more than `tolerance` value.

It is recommended to set both option values to guide the search towards optimum for conflicting objectives, e.g. cost
minimization and any of work balance.

An usage example:

```json
{{#include ../../../../../examples/data/pragmatic/basics/multi-objective.balance-load.problem.json:153:159}}
```

## Default behaviour

By default, decision maker minimizes amount of routes, unassigned jobs and total cost which is equal to the following
definition:

```json
{{#include ../../../../../examples/data/pragmatic/basics/multi-objective.default.problem.json:140:154}}
```

Here, cost minimization is a secondary objective which corresponds to a classical hierarchical objective used
by `Solomon` benchmark.


## Hints

* if you're getting unassigned jobs, want to minimize their count, but not all vehicles are used, then try the following
objective:

```json
"objectives": {
    "primary": [
      {
        "type": "minimize-unassigned"
      }
    ],
    "secondary": [
      {
        "type": "minimize-cost"
      }
    ]
  }
```

* if you're using balancing objective and getting high cost or non-realistic, but balanced routes, try to add a tolerance
and threshold to balancing objective:

```json
"objectives": {
    "primary": [
      {
        "type": "minimize-unassigned"
      },
      {
        "type": "minimize-tours"
      }
    ],
    "secondary": [
      {
        "type": "minimize-cost"
      },
      {
        "type": "balance-distance",
        "options": {
          "tolerance": 0.01,
          "threshold": 0.005
        }
      }
    ]
    }
```

## Related errors

* [E1600 an empty objective specified](../errors/index.md#e1600)
* [E1601 duplicate objective specified](../errors/index.md#e1601)
* [E1602 missing cost objective](../errors/index.md#e1602)


## Examples

Please refer to [examples section](../../../examples/pragmatic/objectives/index.md) to see more examples.
