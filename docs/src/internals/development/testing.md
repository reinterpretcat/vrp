# Testing (WIP)

This article contains a collection of various rules, suggestions and thoughts learned while working on vrp solver project.

## Code style

The first important aspect is a style specific for testing code only. Here the following aspects are considered:

### test organization

- tests are split into different types;
    - unit tests
    - feature (integration) tests
    - discovery tests
    - performance tests
    - documentation tests

- where code is located

  [Official documentation](https://doc.rust-lang.org/book/ch11-03-test-organization.html) suggests to put unit tests to the same file with the code that they’re testing.
  This way you can test private interfaces too. In my opinion, keeping testing code within production decreases
  maintainability of the later and it is better to separate them. So, the project uses the different approach:
    - create a `tests` folder in the crate root. This folder is known by cargo as it is used for integration tests
    - based on test type, create a specific folder inside, e.g. `unit`
    - create a separate file using the pattern `<filename>_test.rs`
    - put it as a child of `tests` folder keeping the same directory tree, e.g. `my_crate/tests/unit/algorithms/clustering/dbscan_test.rc`
    - in production code, include it as a child module using `path` directive:
```rust
#[cfg(test)]
#[path = "../../../tests/unit/algorithms/clustering/dbscan_test.rs"]
mod dbscan_test;
```
The testing module is considered as child module, that's why it has full access to the production private interface.

### what is tested (TODO)

### shared functionality
- fake/stubs
    - fake constraints
- add helper logic for repetable setup/validation functionality
    - simple functions
    - builder pattern
    - should be easily discoverable
        - add separate `helpers` module
- ..

## test types

### unit & component testing
- data-driven testing
    - check exact conditions and verify import
- mocks vs fakes

### feature testing
- user acceptance tests

### importance of discovery tests
- last resort
- requires solution checker
- might be difficult to debug
    - huge output
    - unable to minimize the problem
        - how to research such problems
            - minimize manually
            - disable parallelism
            - try to reduce amount of heuristics used (more predictable)
- proptest library

### regression tests
- very specific use cases which were not handled by unit/component testing due to their complexity
- ideally amount of such tests should be minimized
- can be replaced by discovery tests?

### performance testing
- libraries
    - criterion
- can be run using command ..
- difficult to have results stable
    - no isolation
    - non-determinism

### quality testing
- benchmarks
    - use scientific data
        - CVRP
        - VRPTW
    - challenge: no information about how long it can be run
- script automation


### documentation tests
- very few at the moment as the main focus is on standalone usage, not on a crate


## metrics

### code coverage
- aim to have all significant logic covered
- use generated reports to understand gaps in tested code
- never write tests just to increase code coverage
- 90% is fine

### tests stability
- no random failures
