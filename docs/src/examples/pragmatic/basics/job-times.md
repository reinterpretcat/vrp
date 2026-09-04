# Job time constraints

This example demonstrates how to use the `jobTimes` property to restrict when jobs can be served during a vehicle shift.

## Overview

The `jobTimes` property on a vehicle shift allows you to specify:

- **earliestFirst**: The earliest time the vehicle can arrive at its first job
- **latestLast**: The latest time the vehicle can depart from its last job

This is useful for scenarios such as:
- Enforcing business hours (e.g., deliveries only between 10:00 and 16:00)
- Regulatory compliance (e.g., no service before or after certain times)
- Coordinating with external systems that have time-based availability

## Example

In this example, we have:
- A vehicle shift starting at 08:00 with `jobTimes` set to `earliestFirst: 10:00` and `latestLast: 16:00`
- Two jobs:
  - `assignableJob`: Time window 10:00-16:00 (fits within job times)
  - `unassignableJob`: Time window 08:00-09:00 (ends before `earliestFirst`)

The `unassignableJob` cannot be served because its time window ends at 09:00, but the vehicle cannot start serving jobs until 10:00 (`earliestFirst`).

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/job-times.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/job-times.basic.solution.json}}
```

</p></details>

## Key observations

1. **Vehicle waits at depot**: The vehicle departs at 09:52:30 (not 08:00) to arrive at the first job exactly at 10:00 (`earliestFirst`)
2. **Job assigned within window**: `assignableJob` is served at 10:00-10:05, which satisfies both its own time window and the `jobTimes` constraints
3. **Job rejected**: `unassignableJob` is unassigned because its time window (08:00-09:00) ends before `earliestFirst` (10:00)

## Use cases

### Business hours only
```json
"jobTimes": {
  "earliestFirst": "2019-07-04T09:00:00Z",
  "latestLast": "2019-07-04T17:00:00Z"
}
```

### Morning deliveries only
```json
"jobTimes": {
  "latestLast": "2019-07-04T12:00:00Z"
}
```

### Afternoon start
```json
"jobTimes": {
  "earliestFirst": "2019-07-04T13:00:00Z"
}
```
