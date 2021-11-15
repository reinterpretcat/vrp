# Job clustering

Sometimes, the problem definition has jobs which are close to each other, so it makes sense to serve them together at
the same stop. However, typically, job's service time includes extra time costs such as parking, loading/unloading,
which should be considered only once in the stop. A clustering algorithm supposed to help to schedule jobs realistically
in such scenarios and even in some others like delivery with drones.

## Vicinity clustering

An experimental `vicinity` clustering algorithm is designed to cluster close jobs together and serve them at the same
stop considering such aspects of last mile delivery as parking costs, traveling distance/duration between each job in
the cluster, service time reduction, etc. To use it, specify `clustering` property inside the `plan` with the following
properties:

* `type`: a clustering algorithm name. At the moment, only `vicinity` is supported
* `profile`: specifies routing profile used to calculate commute durations and distances. It has the same properties as
profile on vehicle type.
* `threshold`: specifies various parameters which can control how clusters are built. It has the following properties:
  * `duration`: moving duration limit
  * `distance`: moving distance limit
  * `minSharedTime` (optional): minimum shared time for jobs (non-inclusive)
  * `smallestTimeWindow` (optional): the smallest time window of the cluster after service time shrinking
  * `maxJobsPerCluster` (optional): the maximum amount of jobs per cluster
* `visiting`: specifies job visiting policy type:
  * `return`: after each job visit, driver has to return to stop location
  * `continue`: starting from stop location, driver visits each job one by one, returns to it in the end
* `serving`: specifies a policy for job's service time in the single stop. All policies have a `parking` property
  which specifies how much time has to be reserved at initial parking at stop location. Three policy types are available:
  * `original`: keep original service time
  * `multiplier`: multiplies original service time by fixed `value`
  * `fixed`: uses a new fixed `value` instead of original service time
* `filtering`: specifies job filtering properties. At the moment, it has a single property:
  * `excludeJobIds`: ids of the jobs which should not be clustered with others

An example:

```json
{{#include ../../../../../examples/data/pragmatic/clustering/berlin.vicinity-continue.problem.json:233:249}}
```

In the solution, clustered jobs will have extra properties:

* `tour.stop.parking`: specifies time of the parking
* `tour.stop.activity.commute`: specifies job commute information. It has two properties, `forward` and `backward` which
specify information about activity place visit:
  * `location`: a location before/after place visit
  * `distance`: travelled distance
  * `time`: time when commute occurs

An example:

```json
{{#include ../../../../../examples/data/pragmatic/clustering/berlin.vicinity-continue.solution.json:133:156}}
```


## Limitations

The vicinity clustering functionality has some limitations:

- only jobs with single task can be clustered, but their type, such as pickup or delivery, doesn't matter
- clusters are pre-built using a greedy algorithm which picks the closest by duration job first
- extra constraints puts extra limitations: e.g. priority, order, skills defined on jobs should match in the cluster
- commute distance is not included into statistics


## Examples

Please refer to [examples section](../../../examples/pragmatic/clustering/index.md) to see examples.
