# Multiple routing profiles

This example demonstrates how to use multiple routing profiles: `car` and `truck`.


<details>
    <summary>List of problem locations</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/profiles.basic.locations.json}}
```

</p></details>

<details>
    <summary>Routing matrix for car</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/profiles.basic.matrix.car.json}}
```

</p></details>

<details>
    <summary>Routing matrix for truck</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/profiles.basic.matrix.truck.json}}
```

</p></details>


<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/profiles.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/profiles.basic.solution.json}}
```

</p></details>

<details>
    <summary>Usage with cli</summary><p>

```
vrp-cli pragmatic profiles.basic.problem.json -m profiles.basic.matrix.car.json -m profiles.basic.matrix.truck -o profiles.basic.solution.json
```

</p></details>
