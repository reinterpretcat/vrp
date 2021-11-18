# Vehicle types

A vehicle types are defined by `fleet.types` property and their schema has the following properties:

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
    
    - **shiftTime** (optional): max shift time
    - **maxDistance** (optional): max distance
    - **tourSize** (optional): max amount of activities in the tour (without departure/arrival). Please note, that
      clustered activities are counted as one in case of vicinity clustering.
    - **allowedAreas** (optional): a list of areas where vehicle is allowed to serve jobs. Each area is defined by:
        * _priority_ (optional): an area priority, bigger value - less important. You can use this property to prioritize
        jobs in one area over another.
        * _outerShape_ (required): closed polygon specified by coordinates.

        No area restrictions when omitted.

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
- **dispatch** (optional) a list of dispatch places. When specified, shift start location is not considered as depot and
    vehicle has to navigate first to one of these places to load goods with dispatching constraints.
  Check example [here](../../../examples/pragmatic/basics/dispatch.md).
- **breaks** (optional) a list of vehicle breaks. A break is specified by:
     - `time` (required): time window or interval after which a break should happen (e.g. between 3 or 4 hours after start).
     - `places`: list of alternative places defined by `location` (optional), `duration` (required) and `tag` (optional).
       If location of a break is omitted then break is stick to location of job served before break.
     - `policy` (optional): a break skip policy. Possible values:
       * `skip-if-no-intersection`: allows to skip break if actual tour schedule doesn't intersect with vehicle time window (default)
       * `skip-if-arrival-before-end`: allows to skip break if vehicle arrives before break's time window end.

  Please note that break is a soft constraint and can be unassigned in some cases due to other hard constraints, such
  as time windows. You can control its unassignment weight using specific property on `minimize-unassigned` objective.
  See example [here](../../../examples/pragmatic/basics/break.md)
- **reloads** (optional) a list of vehicle reloads. A reload is a place where vehicle can load new deliveries and unload
    pickups. It can be used to model multi trip routes.
  See examples [here](../../../examples/pragmatic/basics/reload.md).


## Related errors

* [E1300 duplicated vehicle type ids](../errors/index.md#e1300)
* [E1301 duplicated vehicle ids](../errors/index.md#e1301)
* [E1302 invalid start or end times in vehicle shift](../errors/index.md#e1302)
* [E1303 invalid break time windows in vehicle shift](../errors/index.md#e1303)
* [E1304 invalid reload time windows in vehicle shift](../errors/index.md#e1304)
* [E1305 invalid allowed area definition in vehicle limits](../errors/index.md#e1305)
* [E1306 invalid dispatch in vehicle shift](../errors/index.md#e1306)
* [E1307 time and duration costs are zeros](../errors/index.md#e1307)