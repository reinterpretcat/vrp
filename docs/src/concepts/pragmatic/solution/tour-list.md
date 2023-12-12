# Tour list

List of tours is essentially individual vehicle routes. Each tour consists of the following properties:

* **typeId**: id of vehicle type
    ```json
    {{#include ../../../../../examples/data/pragmatic/simple.basic.solution.json:18}}
    ```
* **vehicleId**: id of used vehicle. Id of the vehicle is generated from the tour using pattern `$typeId_sequenceIndex`:
    ```json
    {{#include ../../../../../examples/data/pragmatic/simple.basic.solution.json:17}}
    ```
* **shiftIndex**: vehicle's shift index:
    ```json
    {{#include ../../../../../examples/data/pragmatic/simple.basic.solution.json:19}}
    ```
* **stops**: list of stops. See stop structure below
* **statistic**: statistic of the tour.
    ```json
    {{#include ../../../../../examples/data/pragmatic/simple.basic.solution.json:144:155}}
    ```

## Stop structure

Stop represents a location vehicle has to visit within activities to be performed. It has the following properties:

* **location**: a stop location
* **time** (required): arrival and departure time from the stop
* **distance**: distance traveled since departure from start location
* **load**: (required) vehicle capacity after departure from the stop
* **parking** (optional): parking time. Used only with vicinity clustering.
* **activities** (required): list of activities to be performed at the stop. Each stop can have more than one activity.
    See activity structure below.

Please note, that `location` and `distance` are not required: they are omitted in case of the stop for a required break
which during traveling.

Please check examples [here](../../../examples/pragmatic/basics/break.md).


## Activity structure

An activity specifies work to be done and has the following structure:

* **jobId** (required): id of the job or special id (`departure`, `arrival`, `break`, `reload`)
* **type** (required):  activity type: `departure`, `arrival`, `break`, `reload`, `pickup` or `delivery`
* **location** (optional): activity location. Omitted if stop list has one activity
* **time** (optional): start and end time of activity. Omitted if stop list has one activity
* **jobTag** (optional): a job place tag
* **commute** (optional): commute information. Used only with vicinity clustering.

## Examples

An example of stop with one activity:
```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.solution.json:21:40}}
```

An example of stop with two activities:
```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.solution.json:61:101}}
```
