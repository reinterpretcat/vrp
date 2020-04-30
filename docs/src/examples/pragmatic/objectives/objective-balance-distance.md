# Balance travelled distance

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.balance-distance.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.balance-distance.solution.json}}
```

</p></details>

</br>

<div id="geojson" hidden>
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.balance-distance.solution.geojson}}
</div>

<div id="map"></div>

This objective balances tour distances for all tours:

```json
{{#include ../../../../../examples/data/pragmatic/objectives/berlin.balance-distance.problem.json:1003:1017}}
```

All used vehicles should have total tour distance close to each other.
