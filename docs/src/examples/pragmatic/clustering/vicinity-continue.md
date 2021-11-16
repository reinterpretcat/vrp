# Vicinity clustering with job continuation

This examples demonstrates a `continue` type of visit: jobs are visited one by one with returning to the stop point in
the end.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/clustering/berlin.vicinity-continue.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/clustering/berlin.vicinity-continue.solution.json}}
```

</p></details>

</br>


<div id="geojson" hidden>
{{#include ../../../../../examples/data/pragmatic/clustering/berlin.vicinity-continue.solution.geojson}}
</div>

<div id="map"></div>

As parking time is specified in clustering settings, each stop with clustered job has an extra `parking` property:

```json
{{#include ../../../../../examples/data/pragmatic/clustering/berlin.vicinity-continue.solution.json:54:57}}
```

Here, two minutes are planned for parking car at the stop location.

Activities in such stops have `commute` property which contains information about time, location, distance of commute trip.
The property is split into `forward` and `backward` parts:

```json
{{#include ../../../../../examples/data/pragmatic/clustering/berlin.vicinity-continue.solution.json:122:156}}
```

Here `forward` specifies information how to reach activity and `backward` - how to get back to the stop after the job is
served. Original `time` on activity specifies actual service time.