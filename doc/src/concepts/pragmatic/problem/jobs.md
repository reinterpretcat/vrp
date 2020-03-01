# Job

A job is used to model customer demand, additionally, with different constraints, such as time, skills, etc. A job schema
consists of the following properties:

- **id** (required): an unique job id
- **pickups** (optional): a list of pickup tasks
- **deliveries** (optional): a list of delivery tasks
- **priority** (optional): a job priority. Minimum is 1, higher number means less important job
- **skills** (optional): a list of unique skills


A delivery or pickup list specifies multiple job `tasks` and at least one pickup or delivery task has to be defined.
Each task has the following properties:

- **places** (required): list of possible places from which only one has to be visited
- **demand** (required): a task demand
- **tag** (optional): a job tag


Each `place` consists of the following properties:

- **location** (required): a place location
- **duration** (required): service (operational) time to serve task here
- **times** (optional): time windows

Multiple places on single task can help model variable pickup or delivery location, e.g. visit customer at different location
depending on time of the day.


## Pickup job

Pickup job is a job with only `job.pickups` property specified:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:33:57}}
```

The vehicle picks some `good` at pickup locations, which leads to capacity growth according to `job.pickups.demand` value,
and brings it till the end of the tour. Each pickup task has its own properties such as `demand` and `places`.


## Delivery job

Delivery job is a job with only `job.deliveries` property specified:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:4:32}}
```

The vehicle picks some `goods` at the start stop, which leads to initial capacity growth, and brings it to job's locations,
where capacity is decreased based on `job.deliveries.demand` values. Each delivery task has its own properties such as
`demand` and `places`.


## Pickup and delivery job

Pickup and delivery job is a job with both `job.pickups` and `job.deliveries` properties specified:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:58:92}}
```

The vehicle picks some `goods` at  one or multiple `job.pickups.location`, which leads to capacity growth, and brings
them to one or many `job.deliveries.location`. The job has the following rules:

- all pickup/delivery tasks should be done or none of them.
- assignment order is not defined except all pickups should be assigned before any of deliveries.
- sum of pickup demand should be equal to sum of delivery demand

A good example of such job is a job with more than two places with variable demand:

```json
{{#include ../../../../../examples/json-pragmatic/data/multi-job.basic.problem.json:4:55}}
```

This job contains two pickups and one delivery. Interpretation of such job can be "bring two parcels from two different
places to one single customer".

Another example is one pickup and two deliveries:

```json
{{#include ../../../../../examples/json-pragmatic/data/multi-job.basic.problem.json:56:107}}
```

## Examples

Please refer to [multi jobs example](../../../examples/pragmatic/multi-jobs.md) to see how to specify problem with multiple
pickups and deliveries and for [simple jobs example](../../../examples/pragmatic/simple-jobs.md) to see simple case.