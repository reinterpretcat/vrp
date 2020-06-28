# Analyzing results

In this example, solution is returned in a `pragmatic` format which model is described in details
[here](../concepts/pragmatic/solution/index.md). However, analyzing VRP solution might be a difficult task. That's why
`pragmatic` format supports output in [geojson](https://en.wikipedia.org/wiki/GeoJSON) format which can be simply
visualized in numerous web based front ends, e.g. [geojson.io](http://geojson.io/) or using open source tools such
as `leaflet`:

<div id="geojson" hidden>
{{#include ../../../examples/data/pragmatic/objectives/berlin.default.solution.geojson}}
</div>

<div id="map"></div>

To return solution in `geojson` format, use extra `-g` or `--geo-json` option.

## Jupyter notebooks

You might want to look at [this project](https://github.com/reinterpretcat/vrp-analysis).
It provides some scripts and jupyter notebooks in order to perform deeper analysis of algorithm behaviour.
