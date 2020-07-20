# Balance max load

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.balance-max-load.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.balance-max-load.solution.json}}
```

</p></details>

</br>

<div id="geojson" hidden>
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.balance-max-load.solution.geojson}}
</div>

<div id="map"></div>

This objective balances max load across vehicles:

```json
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.balance-max-load.problem.json:1003:1021}}
```

As `minimize-tours` objective is not set, all available vehicles are used serving `10` jobs per vehicle. Result total
cost is higher than for default objective.
