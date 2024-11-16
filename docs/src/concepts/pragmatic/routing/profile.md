# Routing matrix profiles

In order to solve VRP, you need to specify at least one routing matrix profile.


## Usage

Routing matrix profiles are defined in `fleet.profiles`:

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:135:139}}
```

The `name` must be unique for each matrix profile and it should referenced by `profile.matrix` property defined on vehicle:

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:104:106}}
```

Use `-m` option to pass the matrix:

    vrp-cli solve pragmatic problem.json -m routing_matrix.json -o solution.json

If you don't pass any routing matrix, then [haversine formula](https://en.wikipedia.org/wiki/Haversine_formula) is used to
calculate distances between geo locations. Durations are calculated using speed value defined via `speed` property in
each profile. It is optional, default value is `10` which corresponds to `10m/s`.


## Multiple profiles

In general, you're not limited to one single routing profile. You can define multiple ones and pass their matrices
to the solver:

    vrp-cli solve pragmatic problem.json -m routing_matrix_car.json routing_matrix_truck.json

Make sure that for all profile names in `fleet.profiles` you have the corresponding matrix specified.

See [multiple profiles example](../../../examples/pragmatic/basics/profiles.md).


## Time dependent routing

In order to use this feature, specify more than one routing matrix for each profile with timestamp property set.

