# Objectives

A classical objective function (or simply objective) for VRP is minimization of total cost. However, real life scenarios
require different objective function or even more than one considered simultaneously. That's why the solver has a concept
of multi objective.


## Understanding multi objective structure

A multi objective is defined by `objectives` property which has array of array type and defines some kind of "hierarchical"
objective function where priority of objectives decreases from first to the last element of outer array. Objectives inside
the same inner array have the same priority.


## Available objectives

The solver already provides multiple built-in objectives distinguished by their `type`. All these objectives can be
split into the following groups.

### Cost objectives

These objectives specify how "total" cost of job insertion is calculated:

* `minimize-cost`: minimizes total transport cost calculated for all routes. Here, total transport cost is seen as linear
  combination of total time and distance
* `minimize-distance`: minimizes total distance of all routes
* `minimize-duration`: minimizes total duration of all routes

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
       - `threshold`: a minimum shared jobs to count
       - `distance`:  a minimum relative distance between counts when comparing different solutions.
   This objective is supposed to be on the same level within cost ones.


### Work balance objectives

There are four work balance objectives available:

* `balance-max-load`: balances max load in tour
* `balance-activities`: balances amount of activities performed in tour
* `balance-distance`: balances travelled distance per tour
* `balance-duration`: balances tour durations

Each objective has optional parameters defined by `option` property:
* `threshold`: a target coefficient of variation (scale invariant statistical measure of dispersion) value which specifies
desired minimum balancing level. All values below threshold are considered equal which helps the search algorithm to
optimize conflicting objectives.

It is recommended to set both option values to guide the search towards optimum for conflicting objectives, e.g. cost
minimization and any of work balance.

An usage example:

```json
{{#include ../../../../../examples/data/pragmatic/basics/multi-objective.balance-load.problem.json:154:159}}
```

## Default behaviour

By default, decision maker minimizes amount of unassigned jobs, routes and then total cost. This is equal to the following
definition:

```json
{{#include ../../../../../examples/data/pragmatic/basics/multi-objective.default.problem.json:141:157}}
```

Here, cost minimization is a secondary objective which corresponds to a classical hierarchical objective used
by `Solomon` benchmark.

If at least one job has non-zero value associated, then the following objective is used:

```json
{{#include ../../../../../examples/data/pragmatic/basics/multi-objective.maximize-value.problem.json:143:165}}
```

If order on job task is specified, then it is also added to the list of objectives after `minimize-tours` objective.


## Hints

* pay attention to the order of objectives
* if you're using balancing objective and getting high cost or non-realistic, but balanced routes, try to add a threshold to balancing objective:

```json
"objectives": [
    [
      {
        "type": "minimize-unassigned"
      }
    ],
    [
      {
        "type": "minimize-tours"
      }
    ],
    [
      {
        "type": "minimize-cost"
      },
      {
        "type": "balance-distance",
        "options": {
          "threshold": 0.01
        }
      }
    ]
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
