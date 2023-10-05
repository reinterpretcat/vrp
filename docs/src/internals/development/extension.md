# How to extend solver

This section's intention is to give a very brief explanation of some key concepts which might be needed for adding an extra
feature on top of existing logic. For more detailed information, check the corresponding main crates documentation or
source code

## Constrains & objectives

A Vehicle Routing Problem used to consist of various constraints and objectives functions, such as:
* capacity constraint
* time window constraint
* job's total value maximization
* cost/duration/distance minimization
* etc.

Internally, they can be divided into different groups:
- `hard constraints`: a problem invariants which must be hold. Examples: vehicle capacity, job time window.
- `soft constraints`: a problem variants which should be either maximized or minimized. Examples: job assignment, served job's total value.
- `objectives`: an objective of optimization. Typically, it is hierarchical: first, try to assign all jobs, then reduce the total cost of serving them
- `state`: an auxiliary logic to cache some important state and retrieve it faster during search


Under the hood, a [feature](https://docs.rs/vrp-core/latest/vrp_core/models/struct.Feature.html) concept combines all these groups.
This is based on observation, that many features requires constraint and objective defined at the same time.

## A feature concept

Let's take a brief look at some example: a total job value feature, which purpose to maximize value of assigned jobs.
Here, each job has some associated value (or zero if it is not specified) and the purpose is to maximize it.

The following code below utilizes `FeatureBuilder` to construct the feature:

```rust,no_run,noplayground
{{#include ../../../../vrp-core/src/construction/features/total_value.rs:30:45}}
```

This builder gets:
- a unique feature `name`
- dedicated `objective function` which counts value and prefers solutions where it is maximized
- a dedicated `constraint` which enforces some problem invariants regarding job value (in this case, only for proximity clustering)

Additionally, the builder accepts `FeatureState`. Check existing features for more details.

TODO: expand example