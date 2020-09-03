# Routing matrix format

In general, routing matrix has the following schema:

- `profile` (required for time dependent VRP) is name of vehicle profile
- `timestamp` (optional) a date in RFC3999 for which routing info is applicable. Can be used for time dependent VRP.
- `travelTimes` (required) is square matrix of durations in abstract time units represented via single dimensional array
- `distances` (required) is square matrix of distances in abstract distance unit represented via single dimensional array
- `errorCodes` (optional): must be present if there is no route between some locations. Non-zero value signalizes about
    routing error.

Both durations and distances are mapped to the list of unique locations generated from the problem definition. In this
list, locations are specified in the order they defined. For example, if you have two jobs with locations A and B, one
vehicle type with depot location C, then you have the following location list: A,B,C. It corresponds to the matrix (durations
or distances):

|    |    |    |
|----|----|----|
|  0 | AB | AC |
| BA |  0 | BC |
| CA | CB |  0 |


where
- `0`: zero duration or distance
- `XY`: distance or duration from X location to Y

As single dimensional array it looks like:

    [0,AB,AC,BA,0,BC,CA,CB,0]


`vrp-cli` command provides a helper command to get it as well as `pragmatic` lib exposes method to get the list
pragmatically:

```
vrp-cli solve pragmatic problem.json --get-locations -o locations.json
```

The output format is a simply array of unique geo locations:

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.locations.json}}
```

You can use it to get a routing matrix from any of routing services of your choice, but the order in resulting matrix
should be kept as expected.


Routing matrix example:

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.matrix.json}}
```

If you have already your routing matrix, you can use location indices instead of geocoordinates as described
[here](../routing/index.md#location-format).