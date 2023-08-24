# Recharge stations

This example demonstrates an **experimental** feature to model a simple scenario of `Electric VRP`. Here, the vehicles have
a distance limit (10km), so that they are forced to visit charging stations. Each station is defined by specific location,
charging duration and time windows.

**TODO**: update examples

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/recharge.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/recharge.basic.solution.json}}
```

</p></details>


<div id="geojson" hidden>
{{#include ../../../../../examples/data/pragmatic/basics/recharge.solution.geojson}}
</div>

<div id="map"></div>
