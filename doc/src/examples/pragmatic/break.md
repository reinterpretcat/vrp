# Break

This example demonstrates how to use vehicle break with omitted location and time window.

<details>
    <summary>List of problem locations</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/break.basic.locations.json}}
```

</p></details>

<details>
    <summary>Routing matrix</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/break.basic.matrix.json}}
```

</p></details>


<details>
    <summary>Complete problem json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/break.basic.problem.json}}
```

</p></details>

<details>
    <summary>Complete solution json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/break.basic.solution.json}}
```

</p></details>

<details>
    <summary>Usage with cli</summary><p>

```
vrp-cli pragmatic break.basic.problem.json -m break.basic.matrix.json -o break.basic.solution.json --max-generations=100
```

</p></details>
