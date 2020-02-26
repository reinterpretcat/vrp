# Routing profiles

## Usage

In order to solve VRP, you need to specify at least one routing profile in `fleet.profiles`:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:108:113}}
```

The `name` must be unique for each profile and it should referenced by `profile` property defined on vehicle:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:74:79}}
```

## Multiple profiles

In general, you're not limited to one single routing profile. You can define multiple and pass them to the solver in the
order they are defined in `fleet.profiles`. See [multiple profiles example](../../../examples/pragmatic/profiles.md).