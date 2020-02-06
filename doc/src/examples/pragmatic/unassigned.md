# Unassigned jobs

This example demonstrates one job which is unassigned due to unreachable location.

<details>
    <summary>List of problem locations</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/unassigned.unreachable.locations.json}}
```

</p></details>

<details>
    <summary>Routing matrix</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/unassigned.unreachable.matrix.json}}
```

</p></details>


<details>
    <summary>Complete problem json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/unassigned.unreachable.problem.json}}
```

</p></details>

<details>
    <summary>Complete solution json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/unassigned.unreachable.solution.json}}
```

</p></details>

<details>
    <summary>Usage with cli</summary><p>

```
vrp-cli pragmatic unassigned.unreachable.problem.json -m unassigned.unreachable.matrix.json -o unassigned.unreachable.solution.json --max-generations=100
```

</p></details>
