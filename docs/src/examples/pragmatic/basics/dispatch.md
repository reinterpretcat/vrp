# Vehicles dispatch

This example demonstrates how to use a dispatch different from vehicle start location. The problem specifies two dispatch,
one of those is chosen as next from departure stop. Decision is made based on the following criteria:

* how close dispatch's location to vehicle's start location
* how much time vehicle has to wait till dispatch is open
* how long is dispatch's duration

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