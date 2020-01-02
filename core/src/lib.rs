//! Core crate contains a main buildings blocks for metaheuristic to solve variations of ***Vehicle Routing Problem***.
//!
//!
//! ## Vehicle Routing Problem
//! From wiki:
//! > The vehicle routing problem (VRP) is a combinatorial optimization and integer programming problem
//! > which asks "What is the optimal set of routes for a fleet of vehicles to traverse in order to
//! > deliver to a given set of customers?". It generalises the well-known travelling salesman problem
//! > (TSP).
//! >
//! > Determining the optimal solution to VRP is NP-hard, so the size of problems that can be solved,
//! > optimally, using mathematical programming or combinatorial optimization may be limited.
//! > Therefore, commercial solvers tend to use heuristics due to the size and frequency of real
//! > world VRPs they need to solve.
//!
//! ## Design
//!
//! Although performance is constantly in focus, a main idea behind design is extensibility: the crate
//! aims to support a very wide range of VRP variations known as Rich VRP. This is achieved through
//! various extension points: custom constraints, objective functions, acceptance criteria, etc.
//! More details can be found in child modules.

#[cfg(test)]
#[path = "../tests/helpers/mod.rs"]
#[macro_use]
pub mod helpers;

pub mod construction;
pub mod models;
pub mod refinement;
pub mod utils;
