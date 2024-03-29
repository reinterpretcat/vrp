# Vehicle types

A vehicle types are defined by `fleet.vehicles` property and their schema has the following properties:

- **typeId** (required): a vehicle type id
```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:100}}
```

- **vehicleIds** (required): a list of concrete vehicle ids available for usage.
```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:101:103}}
```

- **profile** (required): a vehicle profile which is defined by two properties:
    - **matrix** (required) : a name of matrix profile
    - **scale** (optional): duration scale applied to all travelling times (default is 1.0)
```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:104:106}}
```

- **costs** (required): specifies how expensive is vehicle usage. It has three properties:
                                     
    - **fixed**: a fixed cost per vehicle tour
    - **time**: a cost per time unit
    - **distance**: a cost per distance unit

- **shifts** (required): specify one or more vehicle shift. See detailed description below.

- **capacity** (required): specifies vehicle capacity symmetric to job demand
```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:130:132}}
```

- **skills** (optional): vehicle skills needed by some jobs
```json
{{#include ../../../../../examples/data/pragmatic/basics/skills.basic.problem.json:131:133}}
```

- **limits** (optional): vehicle limits. There are two:
    
    - **maxDuration** (optional): max tour duration
    - **maxDistance** (optional): max tour distance
    - **tourSize** (optional): max amount of activities in the tour (without departure/arrival). Please note, that
      clustered activities are counted as one in case of vicinity clustering.

An example:

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:99:133}}
``` 

## Shift

Essentially, shift specifies vehicle constraints such as time, start/end locations, etc.:

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:112:129}}
```

At least one shift has to be specified. More than one vehicle shift with different times means that this vehicle can be
used more than once. This is useful for multi day scenarios. An example can be found [here](../../../examples/pragmatic/basics/multi-day.md).

Each shift can have the following properties:

- **start** (required) specifies vehicle start place defined via location, earliest (required) and latest (optional) departure time
- **end** (optional) specifies vehicle end place defined via location, earliest (reserved) and latest (required) arrival time.
    When omitted, then vehicle ends on last job location
- **breaks** (optional) a list of vehicle breaks. There are two types of breaks:
    * __required__: this break is guaranteed to be assigned at cost of flexibility. It has the following properties:
      - `time` (required): a fixed time or time offset interval when the break should happen specified by `earliest` and `latest` properties.
        The break will be assigned not earlier, and not later than the range specified.
      - `duration` (required): duration of the break
    * __optional__: although such break is not guaranteed for assignment, it has some advantages over required break:
      - arbitrary break location is supported
      - the algorithm has more flexibility for assignment
      It is specified by:
      - `time` (required): time window or time offset interval after which a break should happen (e.g. between 3 or 4 hours after start).
      - `places`: list of alternative places defined by `location` (optional), `duration` (required) and `tag` (optional).
        If location of a break is omitted then break is stick to location of a job served before break.
      - `policy` (optional): a break skip policy. Possible values:
        * `skip-if-no-intersection`: allows to skip break if actual tour schedule doesn't intersect with vehicle time window (default)
        * `skip-if-arrival-before-end`: allows to skip break if vehicle arrives before break's time window end.

  Please note that optional break is a soft constraint and can be unassigned in some cases due to other hard constraints, such
  as time windows. You can control its unassignment weight using specific property on `minimize-unassigned` objective.
  See example [here](../../../examples/pragmatic/basics/break.md)

  Additionally, offset time interval requires departure time optimization to be disabled explicitly (see [E1307](../errors/index.md#e1307)).

- **reloads** (optional) a list of vehicle reloads. A reload is a place where vehicle can load new deliveries and unload
    pickups. It can be used to model multi trip routes.
  Each reload has optional and required fields:
    - location (required): an actual place where reload activity happens
    - duration (required): duration of reload activity
    - times (optional): reload time windows
    - tag (optional): a tag which will be propagated back within the corresponding reload activity in solution
    - resourceId (optional): a shared reload resource id. It is used to limit amount of deliveries loaded at this reload.
  See examples [here](../../../examples/pragmatic/basics/reload.md).
- **recharges** (optional, experimental) specifies recharging stations and max distance limit before recharge should happen.
  See examples [here](../../../examples/pragmatic/basics/recharge.md).

## Related errors

* [E1300 duplicated vehicle type ids](../errors/index.md#e1300)
* [E1301 duplicated vehicle ids](../errors/index.md#e1301)
* [E1302 invalid start or end times in vehicle shift](../errors/index.md#e1302)
* [E1303 invalid break time windows in vehicle shift](../errors/index.md#e1303)
* [E1304 invalid reload time windows in vehicle shift](../errors/index.md#e1304)
* [E1306 time and duration costs are zeros](../errors/index.md#e1306)
* [E1307 time offset interval for break  is used with departure rescheduling](../errors/index.md#e1307)
* [E1308 invalid vehicle reload resource](../errors/index.md#e1308)