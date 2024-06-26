//! A core crate contains main buildings blocks for constructing heuristics and metaheuristic
//! to solve rich [`Vehicle Routing Problem`](https://en.wikipedia.org/wiki/Vehicle_routing_problem).
//!
//! # Key points
//!
//! A basic idea of the core crate is to design a library which can be used to solve multiple
//! variations of Vehicle Routing Problem (VRP) known also as a rich VRP. In order to achieve that,
//! it defines essential domain models and implements default metaheuristic with preconfigured
//! properties.
//!
//! Another goal is an intuitive design: it should be relatively easy to start using it without prior
//! knowledge of the domain. That's why the API design does not try to generalize models and
//! implementations in order to develop a general purpose metaheuristic.
//! Check [rosomaxa](https://docs.rs/rosomaxa/latest) crate for more generic models/algorithms.
//!
//!
//! Extra functionality, already developed on top of this crate, is available via following crates:
//!
//! - `vrp-scientific` crate supports VRP variations used in scientific benchmarks
//! - `vrp-pragmatic` crate supports custom json format which can be used to model real world scenarios
//! - `vrp-cli` crate provides a command line interface and static library with all available functionality
//!   provided by the project
//!
//! Meanwhile, the project tries to keep the list of dependencies relatively small, but "Not invented HERE"
//! syndrome should be also avoided.
//!
//! The next sections explain some basic concepts such as types used to model VRP definition,
//! constructive heuristics, metaheuristic, etc. Start exploring them, if you are curious about
//! internal implementation or library extension. It you are looking just for user documentation,
//! check the [`user guide`] documentation.
//!
//! [`user guide`]: https://reinterpretcat.github.io/vrp/
//!
//!
//! # Modeling VRP
//!
//! Model definitions can be split into the following groups:
//!
//! - [`common`] group contains common models: time-specific, location, distance, etc.
//! - [`problem`] group contains VRP definition models: job, vehicle, cost-specific, etc.
//! - [`solution`] group contains models which used to represent a VRP solution: route, tour, activity, etc.
//!
//! Additionally, there are a key concepts such as `Feature` and `GoalContext`.
//!
//! Check corresponding modules for details.
//!
//! [`common`]: ./models/common/index.html
//! [`problem`]: ./models/problem/index.html
//! [`solution`]: ./models/solution/index.html
//!
//!
//! # Constructive heuristic
//!
//! A constructive heuristic is a type of heuristic method which starts with an empty solution and
//! repeatedly extends it until a complete solution is obtained.
//!
//! The crate implements various constructive heuristics in [`construction`] module.
//!
//! [`construction`]: ./construction/index.html
//!
//!
//! # Metaheuristic
//!
//! A metaheuristic is a high-level algorithmic framework that provides a set of guidelines or strategies
//! to develop heuristic optimization algorithms. One of its goals is to guide the search process towards
//! optimal solution.
//!
//! See more details about it in `solver` module.
//!
//!
//! # Examples
//!
//! Detailed examples for some of the VRP variants can be found in `examples` folder.
//!
//! Here, the example shows how to construct default configuration for the solver, override some
//! of the default metaheuristic parameters using fluent interface methods, and run the solver:
//!
//! ```
//! # use vrp_core::models::examples::create_example_problem;
//! # use std::sync::Arc;
//! use vrp_core::prelude::*;
//!
//! // create your VRP problem
//! let problem: Arc<Problem> = create_example_problem();
//! // build solver config using pre-build builder with defaults and then override some parameters
//! let config = VrpConfigBuilder::new(problem.clone())
//!     .prebuild()?
//!     .with_max_time(Some(60))
//!     .with_max_generations(Some(100))
//!     .build()?;
//!
//! // run solver and get the best known solution.
//! let solution = Solver::new(problem, config).solve()?;
//!
//! assert_eq!(solution.cost, 42.);
//! assert_eq!(solution.routes.len(), 1);
//! assert_eq!(solution.unassigned.len(), 0);
//! # Ok::<(), GenericError>(())
//! ```
//!

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

#[macro_use]
pub mod macros;
pub mod prelude;

pub mod algorithms;
pub mod construction;
pub mod models;
pub mod solver;
pub mod utils;

pub use rosomaxa;
