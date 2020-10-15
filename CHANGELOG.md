# Change Log

All notable changes to this project will be documented in this file.

## [Unreleased]

## [v1.6.1] - 2020-10-15

### Added

- `breaks` property in `minimize-unassigned` objective
- dependencies audit by `github actions`

### Changed

- break assignment is made less strict by default. User has certain control on its assignment by the objective.
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

[Unreleased]: https://github.com/reinterpretcat/vrp/compare/v1.6.1...HEAD
[v1.6.1]: https://github.com/reinterpretcat/vrp/compare/v1.6.0...v1.6.1
[v1.6.0]: https://github.com/reinterpretcat/vrp/compare/v1.5.0...v1.6.0
[v1.5.0]: https://github.com/reinterpretcat/vrp/compare/v1.0.0...v1.5.0
