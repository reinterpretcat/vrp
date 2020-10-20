# Vehicles dispatch

The vehicle dispatch feature is useful in the following cases:

* there is a fixed loading time at the beginning of the tour
* vehicles does not start at the depot
* a depot has certain capacity limitation on amount of vehicles loaded simultaneously

When the problem specifies more than one dispatch place, one of those is chosen as next from departure stop.
Decision is made based on the following criteria:

* how close dispatch's location to vehicle's start location
* how much time vehicle has to wait till dispatch is open
* how long is dispatch's duration

## Example

The problem definition has one dispatch place with three different time slots with maximum capacity of one vehicle.
As result, three vehicles are dispatched at different times.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/dispatch.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/dispatch.basic.solution.json}}
```

</p></details>

<div id="geojson" hidden>
{{#include ../../../../../examples/data/pragmatic/basics/dispatch.basic.solution.geojson}}
</div>

<div id="map"></div>