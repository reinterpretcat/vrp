# Unassigned jobs

When job cannot be assigned, it goes to the list of unassigned jobs:

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/unassigned.unreachable.solution.json:113:123}}
```

Each item in this list has job id, reason code and description.


## Reasons of unassigned jobs

| code  | description | possible action |
|-------|-------------|-----------------|
| 0   | `unknown` | - |
| 1   | `cannot serve required skill` | allocate more vehicles with given skill? |
| 2   | `cannot be visited within time window`  | allocate more vehicles, relax time windows, etc.? |
| 3   | `does not fit into any vehicle due to capacity` | allocate more vehicles?  |
| 100 | `location unreachable`  | change job location to routable place? |
| 101 | `cannot be assigned due to max distance constraint of vehicle` | allocate more vehicles?  |
| 102 | `cannot be assigned due to shift time constraint of vehicle`  | allocate more vehicles? |
| 103 | `break is not assignable` | correct break location or/and time window?  |
| 104 | `cannot be served due to relation lock` | review relations?  |
| 105 | `cannot be served due to priority` | allocate more vehicles, relax priorities? |
| 106 | `cannot be assigned due to area constraint` | make sure that jobs inside allowed areas?  |


## Example

An example of problem with unassigned jobs can be found [here](../../../examples/pragmatic/basics/unassigned.md).
