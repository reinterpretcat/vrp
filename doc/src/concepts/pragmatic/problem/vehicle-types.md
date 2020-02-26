# Vehicle types

A vehicle types are defined by `fleet.types` property and their schema has the following properties:

- **id** (required): a vehicle id
```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:77}}
```

- **profile** (required): a name of routing profile
```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:78}}
```

- **costs** (required): specifies how expensive is vehicle usage. It has three properties:
                                     
    - **fixed**: a fixed cost per vehicle tour
    - **time**: a cost per time unit
    - **distance**: a cost per distance unit

- **shifts** (required): specify one or more vehicle shift. See detailed description below.

- **capacity** (required): specifies vehicle capacity symmetric to job demand
```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:102:104}}
```

- **amount** (required): amount of available vehicles of this type

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:105}}
```

- **skills** (optional): vehicle skills needed by some jobs
```json
{{#include ../../../../../examples/json-pragmatic/data/skills.basic.problem.json:36:38}}
```

- **limits** (optional): vehicle limits. There are two:
    
    - **shiftTime**: max shift time
    - **maxDistance**: max distance

An example:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:79:83}}
``` 

## Shift

Essentially, shift specifies vehicle constraints such as time, start/end locations, etc.:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:85:100}}
```

At least one shift has to be specified. More than one vehicle shift with different times means that this vehicle can be
used more than once. This is useful for multi day scenarios. An example can be found [here](../../../examples/pragmatic/multi-day.md).

Each shift can have the following properties:

- **start** (required) specifies vehicle start place defined via location and earliest departure time
- **end** (optional) specifies vehicle end place defined via location and latest arrival time. When omitted, then vehicle
    ends on last job location
- **breaks** (optional) a list of vehicle breaks. A break is specified by:
     - time window or interval after which a break should happen (e.g. between 3 or 4 hours after start).
     - optional location. If it is omitted then break is stick to location of job served before break
    See example [here](../../../examples/pragmatic/break.md)
- **reloads** (optional) a list of vehicle reloads. A reload is a place where vehicle can load new deliveries and unload
    pickups. It can be used to model multi trip routes.
    See examples [here](../../../examples/pragmatic/reload.md).
