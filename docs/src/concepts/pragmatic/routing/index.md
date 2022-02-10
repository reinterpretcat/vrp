# Routing data

In order to solve real life VRP, you need to provide routing information, such as distances and durations between all
locations in the problem. Getting this data is not a part of the solver, you need to use some external service to get it.
Once received, it has to be passed within VRP definition in specific routing matrix format.

When no routing matrix information supplied, the solver uses haversine distance approximation. See more information
about such behavior [here](../../../getting-started/routing.md).


## Location format

Location can be represented as one of two types:

* location as geocoodinate

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:10:13}}
```

* location as index reference in routing matrix

```json
{{#include ../../../../../examples/data/pragmatic/simple.index.problem.json:10:12}}
```

Please note, that you cannot mix these types in one problem definition. Also routing approximation cannot be used with
location indices.


## Related errors

* [E0002 cannot create transport costs](../errors/index.md#e0002)
* [E1500 duplicate profile names](../errors/index.md#e1500)
* [E1501 empty profile collection](../errors/index.md#e1501)
* [E1502 mixing different location types](../errors/index.md#e1502)
* [E1503 location indices requires routing matrix to be specified](../errors/index.md#e1503)
* [E1504 amount of locations does not match matrix dimension](../errors/index.md#e1504)
* [E1505 unknown matrix profile name in vehicle or vicinity clustering profile](../errors/index.md#e1505)
