# Job priorities

There are two types of job priorities:

* **assignment priority**: the solver tries to avoid such jobs to be unassigned by maximizing their total value.
  Assignment priority is modeled by _value_ property on the job and used within the _maximize-value_ objective.
* **order priority**: the solver tries to assign such jobs prior others, close to beginning of the route.
  Order priority is modeled by _order_ property on the job.

## Basic job value example

The example below demonstrates how to use assignment priority by defining _value_ on the jobs. The source problem has a
single vehicle with limited capacity, therefore one job has to be unassigned. The solver is forced to skip the cheapest
one as it has no value associated with it.

Please note, that there is no need to redefine objective to include `maximize-value` one as it will be added automatically
on top of default.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/priorities.value.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/priorities.value.solution.json}}
```

</p></details>

</br>

<div id="geojson" hidden>
{{#include ../../../../../examples/data/pragmatic/basics/priorities.value.solution.geojson}}
</div>

<div id="map"></div>
