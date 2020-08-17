# Routing data

In order to solve real life VRP, you need to provide routing information, such as distances and durations between all
locations in the problem. Getting this data is not a part of the solver, you need to use some external service to get it.
Once received, it has to be passed within VRP definition in specific routing matrix format.

When no routing matrix information supplied, the solver uses haversine distance approximation. See more information
about such behavior [here](../../../getting-started/routing.md).
