# Multi jobs

A multi job is a job with `job.places.pickups` and `job.places.deliveries` properties specified. All these sub-jobs
should be assigned or none of them. The assignment order is not defined except all pickups should be assigned before
any of deliveries.


The main idea of this job type is to model jobs which consist of more than two places with variable demand:


```json
{{#include ../../../../../examples/json-pragmatic/data/multi-job.basic.problem.json:5:46}}
```

This job contains two pickups and one delivery. Interpretation of such job can be "bring two parcels from two different
places to one single customer".

Another example is one pickup and two deliveries:

```json
{{#include ../../../../../examples/json-pragmatic/data/multi-job.basic.problem.json:47:88}}
```

## Example

Please refer to [complete example](../../../examples/pragmatic/multi-jobs.md) to see how to specify problem with multi
jobs.