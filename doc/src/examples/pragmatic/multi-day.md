# Multi day/shift

This example demonstrates how to simulate multi day/shift planning scenario. The problem has jobs with time windows of
different days and one vehicle type with two shifts on different days.


<details>
    <summary>List of problem locations</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-day.basic.locations.json}}
```

</p></details>

<details>
    <summary>Routing matrix</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-day.basic.matrix.json}}
```

</p></details>


<details>
    <summary>Complete problem json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-day.basic.problem.json}}
```

</p></details>

<details>
    <summary>Complete solution json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-day.basic.solution.json}}
```

</p></details>

<details>
    <summary>Usage with cli</summary><p>

```
vrp-cli pragmatic multi-day.basic.problem.json -m multi-day.basic.matrix.json -o multi-day.basic.solution.json --max-generations=100
```

</p></details>
