# Change Log

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

* a limited support for TSPLIB95 format

### Changed

* change defaults for initial solution building logic
* improve rosomaxa algorithm
* improve tour order objective logic
* update dependencies


## [v1.11.4] - 2021-09-10

### Changed

* fix a memory leak
* update dependencies


## [v1.11.3] - 2021-09-07

### Added

* `analyze clusters` command to cli
* break policy property
* a penalty property for skipping break in `maximize-value` objective

### Changed

* improve clustering removal heuristic
* enhance rules for slow search detection and reduce search radius in case of it
* improve unassigned jobs handling logic
* update dependencies


## [v1.11.2] - 2021-08-22

### Added

* `group` constraint on job in pragmatic format


## [v1.11.1] - 2021-08-17

### Added

* slice recreate method

### Changed

* `breaking`: move tag from task to place level in pragmatic format
* `breaking`: adjust break definition to be consistent with job
* update dependencies


## [v1.11.0] - 2021-08-08

### Added

* a metaheuristic which searches in infeasible solution space
* logic to swap objective with some small probability in rosomaxa algorithm

### Changed

* refactor logging configuration
* change metaheuristic coefficients


## [v1.10.8] - 2021-07-21

### Changed

* validation rule for any relation


## [v1.10.7] - 2021-07-12

### Added

* export validation function


## [v1.10.6] - 2021-06-27

### Added

* logging prefix in config

### Changed

* minor changes in core logic


## [v1.10.5] - 2021-06-20

### Changed

* finalize tour order implementation and make it work on job place level
* minor improvements in pragmatic format logic
* update dependencies

### Removed

* undocumented `hre` format


## [v1.10.4] - 2021-05-15

### Added

* pre/post processing steps for problem and solution
* optimization which moves backward departure time
* coefficient of variation termination criteria is extended to support time period

### Changed

* do not always try to move forward departure time


## [v1.10.3] - 2021-05-03

### Added

* new recreate method: `RecreateWithSkipRandom`

### Changed

* `min-cv` can be used in exploration search phase
* improve rosomaxa algorithm


## [v1.10.2] - 2021-04-28

### Changed

* introduce `min-cv` parameter instead of `cost-variation`
* improved stability of some tests
* bug fixes

### Removed

* `cost-variation` parameter


## [v1.10.1] - 2021-04-20

### Changed

* `breaking`: rename job's `priority` property to `order`
* change default objective behaviour when `value` property is used


## [v1.10.0] - 2021-04-02

This release has breaking changes in pragmatic format and internal apis.

### Added

- new objectives: `minimize-distance` and `minimize-duration`
- new CLI option: `init-size` to control amount of initial solutions to be built built
- travelling duration scale on vehicle profile: it can be used to adjust durations for specific vehicle type

### Changed

- optimize cluster ruin method
- improve unassigned code reason handling
- `breaking`: convert profile property on vehicle type to an object

### Removed

- `type` property from matrix profile


## [v1.9.1] - 2021-03-24

### Added

- maximize value objective

### Changed

- build multiple initial solutions


## [v1.9.0] - 2021-03-19

### Changed

- use dynamic hyper-heuristic by default
- flatten objective functions definition
- rebalance coefficients of recreate methods
- reduce default population selection size


## [v1.8.1] - 2021-02-26

### Added

- a new ruin method which destroys closest routes
- more solution checker rules

### Changed

- rebalance ruin methods

### Fixed

- fix an issue with huge amount of possible permutations in multi job


## [v1.8.0] - 2021-02-07

### Added

- a new mutation operator: decompose search which is used for bigger problem instances
- a new population type: greedy
- `breaking`: introduced hyper-heuristic model
- an experimental dynamic selective hyper-heuristic (WIP)

### Changed

- speedup processing of unassigned jobs


## [v1.7.4] - 2021-01-23

### Changed

- introduced parallelism control options (experimental).


## [v1.7.3] - 2021-01-08

### Changed

- update `hre` format support to v2 version
- update dependencies


## [v1.7.2] - 2020-12-05

### Changed

- update dependencies
- apply minor algorithm changes


## [v1.7.1] - 2020-11-29

This release focuses on improving performance and bug fixing.

### Changed

- use `stale` flag internally to avoid route state updates on non-changed routes

### Removed

- remove unstable push tour departure LS method


## [v1.7.0] - 2020-11-23

This release features a new solution space exploration algorithm called ROSOMAXA: Routing Optimizations
with Self Organizing MAps and eXtrAs (pronounced as "rosomaha", from russian "росомаха" - "wolverine").

### Added

- add and use by default rosomaxa algorithm
- add a new LS method to push tour departure in the future
- add a Local Search mutation
- add farthest insertion heuristic

### Changed

- move Local Search out of Ruin Recreate mutation
- `breaking`: use string as unassigned job reason code


## [v1.6.4] - 2020-10-27

### Added

- tour size constraint which limits amount of job activities per tour


## [v1.6.3] - 2020-10-21

### Added

- introduce dispatch feature to support vehicle dispatching functionality (depot replacement)

## Removed

- `breaking`: removed depot feature

## Changed

- `ExchangeIntraRouteRandom` now removes and reinserts with noise a single random job


## [v1.6.2] - 2020-10-18

### Changed

- `breaking`: job skills are now defined by `allOf`, `oneOf` and `noneOf` conditions
- use `hashbrown` library in `pragmatic` crate
- fixed minor bugs

### Removed

- `config` property from `pragmatic` format


## [v1.6.1] - 2020-10-15

### Added

- `breaks` property in `minimize-unassigned` objective
- dependencies audit by `github actions`

### Changed

- break assignment is made less strict by default. User has certain control on its assignment by the objective
- replaced `uniform_real(0., 1.)` usage with `is_hit` method in `Random` trait
- default parameters of local search operators

### Fixed

- minor tech debt


## [v1.6.0] - 2020-10-14

### Added

- cargo features to build `cli` library without certain dependencies
- proper `k-regret` recreate method
- `recreate-with-perturbation` recreate method
- add `local search` operators to ruin recreate mutation:
    - inter route best
    - inter route random
    - intra route random

### Changed

- renamed regret to `skip-best` method

### Fixed

- some issues in init readers


## [v1.5.0] - 2020-09-07

### Added

- accept location indices for routing matrix

### Changed

- do not generate initial solutions when initial solution supplied

### Fixed

- ruin bug with zero-cost jobs
- population size performance issue
- incorrect checker expectations regarding `vehicleId` template


## v1.0.0 - 2020-04-09

- Initial public release


## v0.0.1 - 2019-08-26

- Initial commit

[Unreleased]: https://github.com/reinterpretcat/vrp/compare/v1.11.4...HEAD
[v1.11.4]: https://github.com/reinterpretcat/vrp/compare/v1.11.3...v1.11.4
[v1.11.3]: https://github.com/reinterpretcat/vrp/compare/v1.11.2...v1.11.3
[v1.11.2]: https://github.com/reinterpretcat/vrp/compare/v1.11.1...v1.11.2
[v1.11.1]: https://github.com/reinterpretcat/vrp/compare/v1.11.0...v1.11.1
[v1.11.0]: https://github.com/reinterpretcat/vrp/compare/v1.10.8...v1.11.0
[v1.10.8]: https://github.com/reinterpretcat/vrp/compare/v1.10.7...v1.10.8
[v1.10.7]: https://github.com/reinterpretcat/vrp/compare/v1.10.6...v1.10.7
[v1.10.6]: https://github.com/reinterpretcat/vrp/compare/v1.10.5...v1.10.6
[v1.10.5]: https://github.com/reinterpretcat/vrp/compare/v1.10.4...v1.10.5
[v1.10.4]: https://github.com/reinterpretcat/vrp/compare/v1.10.3...v1.10.4
[v1.10.3]: https://github.com/reinterpretcat/vrp/compare/v1.10.2...v1.10.3
[v1.10.2]: https://github.com/reinterpretcat/vrp/compare/v1.10.1...v1.10.2
[v1.10.1]: https://github.com/reinterpretcat/vrp/compare/v1.10.0...v1.10.1
[v1.10.0]: https://github.com/reinterpretcat/vrp/compare/v1.9.1...v1.10.0
[v1.9.1]: https://github.com/reinterpretcat/vrp/compare/v1.9.0...v1.9.1
[v1.9.0]: https://github.com/reinterpretcat/vrp/compare/v1.8.1...v1.9.0
[v1.8.1]: https://github.com/reinterpretcat/vrp/compare/v1.8.0...v1.8.1
[v1.8.0]: https://github.com/reinterpretcat/vrp/compare/v1.7.4...v1.8.0
[v1.7.4]: https://github.com/reinterpretcat/vrp/compare/v1.7.3...v1.7.4
[v1.7.3]: https://github.com/reinterpretcat/vrp/compare/v1.7.2...v1.7.3
[v1.7.2]: https://github.com/reinterpretcat/vrp/compare/v1.7.1...v1.7.2
[v1.7.1]: https://github.com/reinterpretcat/vrp/compare/1.7.0...v1.7.1
[v1.7.0]: https://github.com/reinterpretcat/vrp/compare/v1.6.4...1.7.0
[v1.6.4]: https://github.com/reinterpretcat/vrp/compare/v1.6.3...v1.6.4
[v1.6.3]: https://github.com/reinterpretcat/vrp/compare/v1.6.2...v1.6.3
[v1.6.2]: https://github.com/reinterpretcat/vrp/compare/v1.6.1...v1.6.2
[v1.6.1]: https://github.com/reinterpretcat/vrp/compare/v1.6.0...v1.6.1
[v1.6.0]: https://github.com/reinterpretcat/vrp/compare/v1.5.0...v1.6.0
[v1.5.0]: https://github.com/reinterpretcat/vrp/compare/v1.0.0...v1.5.0
