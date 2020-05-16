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
//! - [`scientific`] supports VRP variations used in scientific benchmarks
//! - [`pragmatic`] supports custom json format which can be used to model real world scenarios
//! - [`cli`] provides a command line interface and static library with all available functionality
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
//! [`scientific`]: ../vrp_scientific/index.html
//! [`pragmatic`]: ../vrp_pragmatic/index.html
//! [`cli`]: ../vrp_cli/index.html
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
//!

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

pub mod algorithms;
pub mod construction;
pub mod models;
pub mod solver;
pub mod utils;
