# Default behavior: fleet and cost minimization

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.default.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.default.solution.json}}
```

</p></details>

</br>

<div id="geojson" hidden>
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.default.solution.geojson}}
</div>

<div id="map"></div>

By default, the first objective for the solver is to minimize amount of unassigned jobs, then fleet usage, and the last
is total cost minimization:

```json
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.default.problem.json:1004:1014}}
```

As result, solution has minimum amount of vehicles used to serve all jobs (`3`).

Note, that load between these vehicles is not equally distributed as it increases the total cost. 