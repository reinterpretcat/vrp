# Relations

These examples demonstrates how to use relation feature.


## Tour type

In this example, tour relation locks two jobs to specific vehicle in any order.

<details>
    <summary>List of problem locations</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/relation-tour.basic.locations.json}}
```

</p></details>

<details>
    <summary>Routing matrix</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/relation-tour.basic.matrix.json}}
```

</p></details>


<details>
    <summary>Complete problem json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/relation-tour.basic.problem.json}}
```

</p></details>

<details>
    <summary>Complete solution json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/relation-tour.basic.solution.json}}
```

</p></details>

<details>
    <summary>Usage with cli</summary><p>

```
vrp-cli pragmatic relation-tour.basic.problem.json -m relation-tour.basic.matrix.json -o relation-tour.basic.solution.json --max-generations=100
```

</p></details>


## Sequence type

In this example, sequence relation locks two jobs to specific vehicle starting from departure.

<details>
    <summary>List of problem locations</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/relation-strict.basic.locations.json}}
```

</p></details>

<details>
    <summary>Routing matrix</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/relation-strict.basic.matrix.json}}
```

</p></details>


<details>
    <summary>Complete problem json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/relation-strict.basic.problem.json}}
```

</p></details>

<details>
    <summary>Complete solution json</summary><p>

```json
{{#include ../../../../examples/json-pragmatic/data/relation-strict.basic.solution.json}}
```

</p></details>

<details>
    <summary>Usage with cli</summary><p>

```
vrp-cli pragmatic relation-strict.basic.problem.json -m relation-strict.basic.matrix.json -o relation-strict.basic.solution.json --max-generations=100
```

</p></details>
