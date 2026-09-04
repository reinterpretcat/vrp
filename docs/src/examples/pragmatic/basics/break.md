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


## Required break

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

When using `OffsetTime` breaks, the offset is relative to the route cost span anchor: for `depot-to-depot` and
`depot-to-last-job` spans, the anchor is the departure time; for `first-job-to-depot` and `first-job-to-last-job`
spans, the anchor is the first job's arrival time. Flexible start times (where `shift.start.earliest` differs from
`shift.start.latest`) are supported.