# Multiple objectives

This example demonstrates how to use different objectives using `objectives` property.

You can find more details about the topic on [objective concept section](../../concepts/pragmatic/problem/objectives.md).

## Minimize routes, unassigned jobs and total cost with custom goals

```json
{{#include ../../../../examples/json-pragmatic/data/multi-objective.goal.problem.json:138:159}}
```


## Balance max load with total cost minimization

In this example, the primary objective is balancing load across all tours with threshold half of the load. The secondary
objective is cost minimization with desired value and variation coefficient:

```json
{{#include ../../../../examples/json-pragmatic/data/multi-objective.balance-load.problem.json:138:157}}
```


## Balance activities with total cost minimization

In this example, the primary objective is balancing activities across all tours with threshold 4 activities per tour.
The secondary objective is cost minimization with desired value and variation coefficient:

```json
{{#include ../../../../examples/json-pragmatic/data/multi-objective.balance-activities.problem.json:138:157}}
```