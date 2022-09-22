//! This module contains building blocks for constructive heuristics.
//!
//! # Insertion heuristic
//!
//! Insertion heuristic is a popular method to find quickly a **feasible** solution, but without a
//! guarantee of good quality. Essentially, it constructs the solution by repeatedly inserting an
//! unrouted customer into a partially constructed route or as a first customer in an additional
//! route.
//!

pub mod clustering;
pub mod enablers;
pub mod features;
pub mod heuristics;
pub mod probing;
