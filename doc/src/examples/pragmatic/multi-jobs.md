# Multi jobs

These example demonstrates how to use multi jobs. You can find more details about job schema on
[multi job concept section](../../concepts/pragmatic/problem/jobs.md).

## Multiple pickups and deliveries

In this example, there are two multi jobs with slightly different parameters.

<details>
    <summary>List of problem locations</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-job.basic.locations.json}}
```

</p></details>

<details>
    <summary>Routing matrix</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-job.basic.matrix.json}}
```

</p></details>


<details>
    <summary>Complete problem json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-job.basic.problem.json}}
```

</p></details>

<details>
    <summary>Complete solution json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-job.basic.solution.json}}
```

</p></details>

<details>
    <summary>Usage with cli</summary><p>

```
vrp-cli pragmatic multi-job.basic.problem.json -m multi-job.basic.matrix.json -o multi-job.basic.solution.json --max-generations=100
```

</p></details>


## Mixing job types

<details>
    <summary>List of problem locations</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-job.mixed.locations.json}}
```

</p></details>

<details>
    <summary>Routing matrix</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-job.mixed.matrix.json}}
```

</p></details>


<details>
    <summary>Complete problem json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-job.mixed.problem.json}}
```

</p></details>

<details>
    <summary>Complete solution json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/multi-job.mixed.solution.json}}
```

</p></details>

<details>
    <summary>Usage with cli</summary><p>

```
vrp-cli pragmatic multi-job.mixed.problem.json -m multi-job.mixed.matrix.json -o multi-job.mixed.solution.json --max-generations=100
```

</p></details>