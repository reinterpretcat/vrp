# Simple jobs

This example demonstrates how to use simple jobs. It has one delivery, one pickup, and one pickup and delivery job with
one dimensional demand. 

You can find more details about job schema on [simple job concept section](../../concepts/pragmatic/problem/jobs.md).

<details>
    <summary>List of problem locations</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/simple.basic.locations.json}}
```

</p></details>

<details>
    <summary>Routing matrix</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/simple.basic.matrix.json}}
```

</p></details>


<details>
    <summary>Complete problem json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/simple.basic.problem.json}}
```

</p></details>

<details>
    <summary>Complete solution json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/simple.basic.solution.json}}
```

</p></details>

<details>
    <summary>Usage with cli</summary><p>

```
vrp-cli pragmatic simple.basic.problem.json -m simple.basic.matrix.json -o simple.basic.solution.json --max-generations=100
```

</p></details>
