# Relations

Relation is a mechanism to lock jobs to specific vehicles. List of relations is a part of `plan` schema and each relation
has the following properties:

- **type** (required): one of three relation types: tour, fixed, or sequence. See description below.
- **vehicleId** (required): a specific vehicle id
- **jobs** (required): list of job ids including reserved: `departure`, `arrival`, `break` and `reload`
- **shiftIndex** (optional): a vehicle shift index.

You can use more than one relation per vehicle.

## Tour type

A `tour` relation is used to lock specific jobs to certain vehicle in any order:

```json
{{#include ../../../../../examples/json-pragmatic/data/relation-tour.basic.problem.json:67:71}}
```

## Fixed type

A `fixed` relation is used to lock specific jobs to certain vehicle in fixed order allowing insertion of new jobs in
between.

## Sequence type

In contrast to `fixed` relation, `sequence` locks jobs to certain vehicle without ability to insert new jobs in between:

```json
{{#include ../../../../../examples/json-pragmatic/data/relation-strict.basic.problem.json:67:71}}
```


## Examples

Please refer to [complete example](../../../examples/pragmatic/relations.md) to see how to specify problem with relations.
