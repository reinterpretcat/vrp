# Default behavior: fleet and cost minimization

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/objectives/berlin.default.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/objectives/berlin.default.solution.json}}
```

</p></details>

</br>

<div id="geojson" hidden>
{{#include ../../../../../examples/json-pragmatic/data/objectives/berlin.default.solution.geojson}}
</div>

<div id="map"></div>

By default, primary objective for the solver is to minimize fleet usage and amount of unassigned jobs, the secondary
objective is total cost minimization:

```json
{{#include ../../../../../examples/json-pragmatic/data/objectives/berlin.default.problem.json:1003:1017}}
```

As result, solution has minimum amount of vehicles used to serve all jobs (`3`).

Note, that load between these vehicles is not equally distributed as it increases the total cost. 