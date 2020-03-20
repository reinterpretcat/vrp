# Tour list

List of tours is essentially individual vehicle routes. Each tour consists of the following properties:

* **typeId**: id of vehicle type
    ```json
    {{#include ../../../../../examples/json-pragmatic/data/simple.basic.solution.json:16}}
    ```
* **vehicleId**: id of used vehicle. Id of the vehicle is generated from the tour using pattern `$typeId_sequenceIndex`:
    ```json
    {{#include ../../../../../examples/json-pragmatic/data/simple.basic.solution.json:15}}
    ```
* **shiftIndex**: vehicle's shift index:
    ```json
    {{#include ../../../../../examples/json-pragmatic/data/simple.basic.solution.json:17}}
    ```
* **stops**: list of stops. See stop structure below
* **statistic**: statistic of the tour.
    ```json
    {{#include ../../../../../examples/json-pragmatic/data/simple.basic.solution.json:140:150}}
    ```

## Stop structure

Stop represents a location vehicle has to visit within activities to be performed. It has the following properties:

* **location**: a stop location
* **time**: arrival and departure time from the stop
* **distance**: distance traveled since departure from start location
* **load**: vehicle capacity after departure from the stop
* **activities**: list of activities to be performed at the stop. Each stop can have more than one activity.
    See activity structure below.

## Activity structure

An activity specifies work to be done and has the following structure:

* **jobId**: id of the job or special id (`departure`, `arrival`, `break`, `reload`)
* **type**:  activity type: `departure`, `arrival`, `break`, `reload`, `pickup` or `delivery`
* **location** (optional): activity location. Omitted if stop list has one activity
* **time** (optional): start and end time of activity. Omitted if stop list has one activity
* **tag** (optional): a job place tag

## Examples

An example of stop with one activity:
```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.solution.json:19:38}}
```

An example of stop with two activities:
```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.solution.json:59:98}}
```
