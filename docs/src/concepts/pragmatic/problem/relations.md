# Relations

Relation is a mechanism to lock jobs to specific vehicles. List of relations is a part of `plan` schema and each relation
has the following properties:

- **type** (required): one of three relation types: tour, fixed, or sequence. See description below.
- **vehicleId** (required): a specific vehicle id
- **jobs** (required): list of job ids including reserved: `departure`, `arrival`, `break` and `reload`
- **shiftIndex** (optional): a vehicle shift index. If not specified, a first, zero indexed, shift assumed

You can use more than one relation per vehicle.


## Any type

A `any` relation is used to lock specific jobs to certain vehicle in any order:

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/relation-any.basic.problem.json:82:89}}
```


## Sequence type

A `sequence` relation is used to lock specific jobs to certain vehicle in fixed order allowing insertion of new jobs in
between.


## Strict type

In contrast to `sequence` relation, `strict` locks jobs to certain vehicle without ability to insert new jobs in between:

```json
{{#include ../../../../../examples/json-pragmatic/data/basics/relation-strict.basic.problem.json:82:89}}
```

In this example, new jobs can be inserted only after job with id `job1`.


## Important notes

Please consider the following notes:

* jobs specified in `sequence` and `strict` are not checked for constraint violations. This might lead to
non-feasible solutions (e.g. routes with capacity or time window violation).

* relation with jobs which have multiple pickups or deliveries are not yet supported


## Related errors

* [E1200 relation has job id which does not present in the plan](../errors/index.md#e1200)
* [E1201 relation has vehicle id which does not present in the fleet](../errors/index.md#e1201)
* [E1202 relation has empty job id list](../errors/index.md#e1202)
* [E1203 strict or sequence relation has job with multiple places or time windows](../errors/index.md#e1203)


## Examples

Please refer to [complete example](../../../examples/pragmatic/basics/relations.md) to see how to specify problem with relations.
