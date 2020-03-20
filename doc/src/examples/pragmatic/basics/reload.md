# Multiple trips

These examples demonstrates how to use vehicle reload feature which is designed to overcome vehicle capacity limitation
in order to perform multiple trips (tours).

Essentially, reload is a place where vehicle can unload `static` pickups and load new `static` deliveries. Here, static
correspond to `static demand` concept which is defined via standalone pickup or delivery jobs, not by single pickup and
delivery job.

## Same location reload

In this scenario, once some jobs are delivered, the vehicle returns to the original depot to load next goods.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/reload.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/reload.basic.solution.json}}
```

</p></details>


## Multiple reloads with different locations

In this scenario, vehicle picks goods and flushes them on two different locations during single tour. This can be used
to model _waste collection_ use case.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/reload.multi.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/reload.multi.solution.json}}
```

</p></details>

