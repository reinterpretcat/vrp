# Area objective

This example demonstrates how to use areas for vehicles.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/area-order.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/basics/area-order.solution.json}}
```

</p></details>

</br>

<div id="geojson" hidden>
{{#include ../../../../../examples/data/pragmatic/basics/area-order.solution.geojson}}
</div>

<div id="map"></div>


To use the feature, you need to specify the following:


## Areas with job ids

Areas defined by unique id and job ids. The example problem has two areas specified in the plan:

```json
{{#include ../../../../../examples/data/pragmatic/basics/area-order.problem.json:3:20}}
```

Please note, that different areas can specify the same job ids. This allows some extra flexibility for assignment.


## Vehicle area limits

Each vehicle has its own area limits which have references to areas specified in the plan, e.g.:

```json
{{#include ../../../../../examples/data/pragmatic/basics/area-order.problem.json:281:290}}
```

## Area order objective

You need to specify an `area-order` objective somewhere in the list of objectives with some extra options. In the example
problem, it is specified at top and allows area order violations:

```json
{{#include ../../../../../examples/data/pragmatic/basics/area-order.problem.json:300:321}}
```

Internally, it adds two extra objectives. One counts total value of the jobs served in each area, the second one counts
violations of the order:

    [1s] population state (phase: exploration, speed: 1181.90 gen/sec, improvement ratio: 0.004:0.000):
         rank: 0, fitness: (-60.000, 0.000, 0.000, 2.000, 170.856), improvement: 0.000%
         rank: 1, fitness: (-60.000, 0.000, 0.000, 2.000, 171.800), improvement: 0.553%
    [1s] generation 2100 took 0ms, rank: 0, fitness: (-60.000, 0.000, 0.000, 2.000, 170.856), improvement: 0.000%

As we specified `"isConstrained": false,` and `isValuePreferred": true`, the `value` objective is the first one (`-60.000`),
followed by the `violations` objective (`0.000`).
