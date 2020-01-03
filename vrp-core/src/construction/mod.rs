//! Building blocks for arbitrary insertion heuristic to construct a feasible solution.
//!
//!
//! # Insertion heuristic
//!
//! Insertion heuristic is a popular method to find quickly a **feasible** solution, but without a
//! guarantee of good quality. Essentially, it constructs the solution by repeatedly inserting an
//! unrouted customer into a partially constructed route or as a first customer in an additional
//! route.

pub mod constraints;
pub mod heuristics;
pub mod states;
