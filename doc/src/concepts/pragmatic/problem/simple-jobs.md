# Simple jobs

A simple job is a job which has single `job.demand` property specified. There are three types of simple jobs.


## Pickup job

Pickup job is a job with only `job.places.pickup` property specified:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:5:29}}
```

The vehicle picks some `good` at pickup location, which leads to capacity growth according to `job.demand` value,
and brings it till the end of the tour.


## Delivery job

Delivery job is a job with only `job.places.delivery` property specified: 

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:30:50}}
```

The vehicle picks some `good` at the start stop, which leads to initial capacity growth, and brings it to job's location,
where capacity is decreased based on `job.demand` value.


## Pickup and delivery job

Pickup and delivery job is a job with both `job.places.pickup` and `job.places.delivery` properties specified:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:51:72}}
```

The vehicle picks some `good` at `job.pickup.location`, which leads to capacity growth, and brings it to the `job.delivery.location`.

### Example

Please refer to [complete example](../../../examples/pragmatic/simple-jobs.md) to see how to specify problem with simple
jobs.

