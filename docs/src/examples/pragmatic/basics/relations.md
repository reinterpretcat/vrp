# Relations

These examples demonstrates how to use relation feature.


## Relation of Any type

In this example, `any` relation locks two jobs to specific vehicle in any order.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/relation-any.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/relation-any.basic.solution.json}}
```

</p></details>


## Relation of Strict type

In this example, `strict` relation locks two jobs to specific vehicle starting from departure.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/relation-strict.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/relation-strict.basic.solution.json}}
```

</p></details>
