# Prototyping and experimenting

As it is already mentioned, the project supports multiple VRP variants for real life scenarios. However,
it might be difficult to start using the solver due to missing routing information or tool for result analysis.

A few tips below show how quick prototyping and experimenting on solver behavior can be simplified.


## Routing matrix approximation

In general, the solver requires routing matrix which is a separate dependency. That's why `pragmatic` format supports distance
approximation using [haversine formula](https://en.wikipedia.org/wiki/Haversine_formula) within fixed speed for durations.
This helps you quickly check how solver works on specific problem variant without need to acquire routing matrix.

To use this feature, simply omit `-m` parameter for routing matrix files.


## Geojson visualization

Analyzing VRP solution might be a difficult task. `pragmatic` format supports output in [geojson](https://en.wikipedia.org/wiki/GeoJSON)
format which can be simply visualized in numerous web based front ends, e.g. [geojson.io](http://geojson.io/).

To return solution in `geojson` format, use `-g` or `--geo-json` option.
