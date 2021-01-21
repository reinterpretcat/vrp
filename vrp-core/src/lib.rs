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
//! check! the [`user guide`] documentation.
//!
//! [`user guide`]: https://reinterpretcat.github.io/vrp/
//!
//!
//! # Modeling VRP
//!
//! Model definitions can be split into three groups:
//!
//! - [`common`] group contains common models: time-specific, location, distance, etc.
//! - [`problem`] group contains VRP definition models: job, vehicle, cost-specific, etc.
//! - [`solution`] group contains models which used to represent a VRP solution: route, tour, activity, etc.
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
//! See more details about it in [`solver`] module.
//!
//! [`solver`]: ./solver/index.html
//!
//!
//! # Examples
//!
//! The most simple way to run solver is to use [`Builder`](./struct.Builder.html). You can tweak
//! metaheuristic parameters by calling corresponding methods of the builder instance:
//!
//! ```
//! # use vrp_core::models::examples::create_example_problem;
//! # use std::sync::Arc;
//! use vrp_core::solver::Builder;
//! use vrp_core::models::Problem;
//! use vrp_core::utils::Environment;
//!
//! // create your VRP problem
//! let problem: Arc<Problem> = create_example_problem();
//! // build solver to run 10 secs or 1000 generation
//! let solver = Builder::new(problem, Environment::default())
//!     .with_max_time(Some(10))
//!     .with_max_generations(Some(1000))
//!     .build()?;
//! // run solver and get the best known solution within its cost.
//! let (solution, cost, _) = solver.solve()?;
//!
//! assert_eq!(cost, 42.);
//! assert_eq!(solution.routes.len(), 1);
//! assert_eq!(solution.unassigned.len(), 0);
//! # Ok::<(), String>(())
//! ```
//!

#![warn(missing_docs)]

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

pub mod algorithms;
pub mod construction;
pub mod models;
pub mod solver;
pub mod utils;
