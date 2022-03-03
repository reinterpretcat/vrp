# Vehicle break

In general, there are two break types: optional and required.


## Optional break

This example demonstrates how to use optional vehicle break with time window and omitted location.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/break.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/break.basic.solution.json}}
```

</p></details>


## Required break (experimental)

This example demonstrates how to use required vehicle break which has to be scheduled at specific time during travel
between two stops.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/break.required.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/break.required.solution.json}}
```

</p></details>

Please note, that departure rescheduling is disabled by setting `shift.start.earliest` equal to `shift.start.latest`.
At the moment, this is a hard requirement when such break type is used.