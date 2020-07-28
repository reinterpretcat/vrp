# Variable depot

This example demonstrates how to use a depot different from vehicle start location. The problem specifies two depots,
one of those is chosen as next from departure stop. Decision is made based on the following criteria:

* how close depot's location to vehicle's start location
* how much time vehicle has to wait till depot is open
* how long is depot's duration

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/depot.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/depot.basic.solution.json}}
```

</p></details>

<div id="geojson" hidden>
{{#include ../../../../../examples/data/pragmatic/basics/depot.basic.solution.geojson}}
</div>

<div id="map"></div>