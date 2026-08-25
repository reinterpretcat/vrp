# Heuristic research playground

This crate runs vector-function and VRP experiments and visualizes the population, objective convergence,
hyper-heuristic telemetry, and GSOM topology over generations.


## Build

```bash
wasm-pack build --target web

# install web server (or use any other)
cargo install basic-http-server
basic-http-server

# open http://127.0.0.1:4000/www/
```

The solver runs in a web worker, so the page can report progress and cancel a run without freezing its controls.
Experiments use the requested generation limit without an additional hidden wall-time cutoff.
Long runs retain roughly 250 population/GSOM snapshots and two dozen larger VRP footprint snapshots. The generation
slider resolves to the nearest retained snapshot. Dynamic-heuristic telemetry similarly retains complete posterior
banks at an adaptive interval rather than every raw operator call.

## Reading operator statistics

The operator tabs separate the independent `best` and `diverse` Thompson-sampling banks. Calls, empirical success
rates, and mean durations use cumulative counters captured at complete posterior checkpoints, so downsampling does not
change their totals. Select a recent generation window to inspect changing operator behavior during stagnation, or use
the cumulative view for global statistics from the start of search through the generation picker. Chart captions report
the actual checkpoint or checkpoint interval used.

An operator success means that its child strictly improved the common pre-batch incumbent according to the configured
objective order. Multiple parallel children can therefore be successful in one generation; the statistic measures the
feedback learned by Thompson sampling, not the number of incumbent replacements accepted into the population. The
posterior chart shows the learned success-probability estimate, while duration remains diagnostic and is not part of
the solution reward. The posterior mean includes expert prior evidence, bounded recent evidence, and stagnation resets;
it can therefore differ from the empirical success rate for the selected interval. Thompson sampling draws from the
posterior distribution rather than always choosing its largest mean, so uncertainty continues to influence allocation.

## Reading the GSOM views

The population map shows objective planes and the neighbor-distance, hit, and error matrices for the selected generation. Fitness
arrows point from a worse occupied node towards a better neighbor; a red cross marks an occupied node with no better
four-neighbor and is used as a local-basin proxy. In exploitation the network is inactive, so the UI retains and labels
the last exploration map instead of displaying a misleading blank map.

For VRP experiments a compact left-hand surface retains the aggregate directed-edge footprint as a coarse reminder of
population structure, while GSOM receives most of the canvas. Peaks show how many stored population members use each
directed edge. Its axes are location identifiers rather than spatial coordinates, so apparent geometric proximity is
not meaningful.

The GSOM evolution tab separates several concerns which a single node count hides:

- total versus occupied nodes indicates storage utilization;
- recently hit nodes and their ratio indicate how broadly selection/training still touches the map;
- local fitness sinks approximate how many distinct promising regions the topology exposes;
- bounding-box density reveals sparse expansion or fragmentation;
- map MSE and learning rate show representation error and adaptation pressure;
- sudden node-count drops expose compaction events and their effect on occupied regions and basins.

These are diagnostics, not optimization objectives. In particular, more basin proxies can also mean fragmentation or
low-quality isolated nodes; interpret them together with the best-fitness curve and objective planes.
