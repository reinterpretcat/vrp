# Routing profiles

## Usage

In order to solve VRP, you need to specify at least one routing profile in `fleet.profiles`:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:131:136}}
```

The `name` must be unique for each profile and it should referenced by `profile` property defined on vehicle:

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:102}}
```

Use `-m` option to pass the matrix:

    vrp-cli solve pragmatic problem.json -m routing_matrix.json -o solution.json


## Multiple profiles

In general, you're not limited to one single routing profile. You can define multiple ones and pass their matrices
to the solver:

    vrp-cli solve pragmatic problem.json -m routing_matrix_car.json -m routing_matrix_truck.json

Make sure that for all profile names in `fleet.profiles` you have the corresponding matrix specified.

See [multiple profiles example](../../../examples/pragmatic/basics/profiles.md).


## Time dependent routing

In order to use this feature, specify more than one routing matrix for each profile with timestamp property set.


## Related errors

* [E1500 duplicate profile names](../errors/index.md#e1500)
* [E1501 empty profile collection](../errors/index.md#e1501)
