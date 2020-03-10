# Job

A job is used to model customer demand, additionally, with different constraints, such as time, skills, etc. A job schema
consists of the following properties:

- **id** (required): an unique job id
- **pickups** (optional): a list of pickup tasks
- **deliveries** (optional): a list of delivery tasks
- **replacements** (optional): a list of replacement tasks
- **services** (optional): a list of service tasks
- **priority** (optional): a job priority. Minimum is 1, higher number means less important job
- **skills** (optional): a list of unique skills


A delivery, pickup, replacement and service lists specify multiple job `tasks` and at least one of such tasks has to be
defined. Each task has the following properties:

- **places** (required): list of possible places from which only one has to be visited
- **demand** (optional/required): a task demand. It is required for all job types, except service
- **tag** (optional): a job tag


Each `place` consists of the following properties:

- **location** (required): a place location
- **duration** (required): service (operational) time to serve task here
- **times** (optional): time windows

Multiple places on single task can help model variable job location, e.g. visit customer at different location
depending on time of the day.


## Pickup job

Pickup job is a job with `job.pickups` property specified,   without `job.deliveries`:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:33:57}}
```

The vehicle picks some `good` at pickup locations, which leads to capacity growth according to `job.pickups.demand` value,
and brings it till the end of the tour. Each pickup task has its own properties such as `demand` and `places`.


## Delivery job

Delivery job is a job with `job.deliveries` property specified, without `job.pickups`:

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

The vehicle picks some `goods` at one or multiple `job.pickups.location`, which leads to capacity growth, and brings
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


## Replacement job

A replacement job is a job with `job.replacement` property specified:

```json
{{#include ../../../../../examples/json-pragmatic/data/multi-job.mixed.problem.json:4:28}}
```

It models an use case when something big has to be replaced at the customer's location. This task requires a new `good`
to be loaded at the beginning of the journey and old replaced one brought to journey's end.


## Service job

A service job is a job with `job.service` property specified:

```json
{{#include ../../../../../examples/json-pragmatic/data/multi-job.mixed.problem.json:29:54}}
```

This job models some work without demand (e.g. handyman visit).


## Mixing job tasks

You can specify multiple tasks properties to get some mixed job:

```json
{{#include ../../../../../examples/json-pragmatic/data/multi-job.mixed.problem.json:55:118}}
```

Similar pickup and delivery job, all these tasks has to be executed or none of them. The order is not specified except
pickups must be scheduled before any delivery, replacement or service.


## Examples

Please refer to [multi jobs example](../../../examples/pragmatic/multi-jobs.md) to see how to specify problem with multiple
pickups and deliveries and for [simple jobs example](../../../examples/pragmatic/simple-jobs.md) to see simple case.