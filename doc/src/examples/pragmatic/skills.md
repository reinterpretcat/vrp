# Skills

This example demonstrates how to use skills feature with jobs and vehicles.

<details>
    <summary>List of problem locations</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/skills.basic.locations.json}}
```

</p></details>

<details>
    <summary>Routing matrix</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/skills.basic.matrix.json}}
```

</p></details>


<details>
    <summary>Complete problem json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/skills.basic.problem.json}}
```

</p></details>

<details>
    <summary>Complete solution json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/skills.basic.solution.json}}
```

</p></details>

<details>
    <summary>Usage with cli</summary><p>

```
vrp-cli pragmatic skills.basic.problem.json -m skills.basic.matrix.json -o skills.basic.solution.json --max-generations=100
```

</p></details>
