# Objectives

A classical objective function (or simply objective) for VRP is minimization of total cost. However, real life scenarios
require different objective function or even more than one considered simultaneously. That's why the solver has a concept
of multi objective.


## Understanding multi objective structure

A multi objective is defined by `objectives` property which has array of objectives and defines lexicographical ordered
objective function. Here, priority of objectives decreases from first to the last element of the array. For the same
priority (or in other words, competitive) objectives, a special `multi-objective` type can be used.


## Available objectives

The solver already provides multiple built-in objectives distinguished by their `type`. All these objectives can be
split into the following groups.

### Cost objectives

These objectives specify how "total" cost of job insertion is calculated:

* `minimize-cost`: minimizes total transport cost calculated for all routes. Here, total transport cost is seen as linear
  combination of total time and distance
* `minimize-distance`: minimizes total distance of all routes
* `minimize-duration`: minimizes total duration of all routes
* `compact-tour`: tries to keep tours compact. Uses one of the other cost objectives internally.

One of these objectives has to be set and only one.

### Scalar objectives

Besides cost objectives, there are other objectives which are targeting for some scalar characteristic of solution:

* `minimize-unassigned`: minimizes amount of unassigned jobs. Although, solver tries to minimize amount of
unassigned jobs all the time, it is possible that solution, discovered during refinement, has more unassigned jobs than
previously accepted. The reason of that can be conflicting objective (e.g. minimize tours) and restrictive
constraints such as time windows. The objective has the following optional parameter:
    * `breaks`: a multiplicative coefficient to make breaks more preferable for assignment. Default value is 1. Setting
     this parameter to a value bigger than 1 is useful when it is highly desirable to have break assigned but its
     assignment leads to more jobs unassigned.
* `minimize-tours`: minimizes total amount of tours present in solution
* `maximize-tours`: maximizes total amount of tours present in solution
* `minimize-arrival-time`: prefers solutions where work is finished earlier
* `fast-service`: prefers solutions when jobs are served early in tours. Optional parameter:
  *  `tolerance`: an objective tolerance specifies how different objective values have to be to consider them different.
      Relative distance metric is used.
* `hierarchical-areas`: an experimental objective to play with clusters of jobs. Internally uses distance minimization as
  a base penalty.
  * `levels` - number of hierarchy levels

### Job distribution objectives

These objectives provide some extra control on job assignment:

* `maximize-value`: maximizes total value of served jobs. It has optional parameters:
    * `reductionFactor`: a factor to reduce value cost compared to max routing costs
    * `breaks`: a value penalty for skipping a break. Default value is 100.
* `tour-order`: controls desired activity order in tours
    * `isConstrained`: violating order is not allowed, even if it leads to less assigned jobs (default is true).
* `compact-tour`: controls how tour is shaped by limiting amount of shared jobs, assigned in different routes,
    for a given job' neighbourhood. It has the following mandatory parameters:
   *  `options`: options to relax objective:
       - `jobRadius`: a radius of neighbourhood, minimum is 1
       - `threshold`: a minimum shared jobs to count
       - `distance`:  a minimum relative distance between counts when comparing different solutions.
   This objective is supposed to be on the same level within cost ones.


### Work balance objectives

There are four work balance objectives available:

* `balance-max-load`: balances max load in tour
* `balance-activities`: balances amount of activities performed in tour
* `balance-distance`: balances travelled distance per tour
* `balance-duration`: balances tour durations

Typically, you need to use these objective with one from the cost group combined under single `multi-objective`.

An usage example:

```json
{{#include ../../../../../examples/data/pragmatic/basics/multi-objective.balance-load.problem.json:148:161}}
```

## Default behaviour

By default, decision maker minimizes the number of unassigned jobs, routes and then total cost. This is equal to the
following definition:

```json
{{#include ../../../../../examples/data/pragmatic/basics/multi-objective.default.problem.json:141:151}}
```

Here, cost minimization is a secondary objective that corresponds to a classical hierarchical objective used, for example,
by `Solomon` benchmark.

If at least one job has non-zero value associated, then the following objective is used:

```json
{{#include ../../../../../examples/data/pragmatic/basics/multi-objective.maximize-value.problem.json:143:156}}
```

If order on job task is specified, then it is also added to the list of objectives after `minimize-tours` objective.


## Hints

* pay attention to the order of objectives
* if you're using balancing objective and getting high cost or non-realistic, but balanced routes, try to use multi-objective:

```json
"objectives": [
  {
    "type": "minimize-unassigned"
  },
  {
    "type": "minimize-tours"
  },
  {
    "type": "multi-objective",
    "strategy": {
      "name": "sum"
    },
    "objectives": [
      {
        "type": "minimize-cost"
      },
      {
        "type": "balance-max-load"
      }
    ]
  }
]
```

## Related errors

* [E1600 an empty objective specified](../errors/index.md#e1600)
* [E1601 duplicate objective specified](../errors/index.md#e1601)
* [E1602 missing one of cost objectives](../errors/index.md#e1602)
* [E1603 redundant value objective](../errors/index.md#e1603)
* [E1604 redundant tour order objective](../errors/index.md#e1604)
* [E1605 value or order of a job should be greater than zero](../errors/index.md#e1605)
* [E1606 multiple cost objectives specified](../errors/index.md#e1606)
* [E1607 missing value objective](../errors/index.md#e1607)


## Examples

Please refer to [examples section](../../../examples/pragmatic/objectives/index.md) to see more examples.
