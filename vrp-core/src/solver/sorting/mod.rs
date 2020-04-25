//! This module contains a logic for processing multiple solutions and multi objective optimization
//! based on NSGA2 algorithm.
//!
//! A NSGA2 implementation is based on the source code from the following repos:
//!
//! https://github.com/mneumann/dominance-ord-rs
//! https://github.com/mneumann/non-dominated-sort-rs
//! https://github.com/mneumann/nsga2-rs
//!
//! which is released under MIT License (MIT), copyright (c) 2016 Michael Neumann
//!

use crate::solver::objectives::MultiObjective;

mod crowding_distance;
pub use self::crowding_distance::*;

mod non_dominated_sort;
pub use self::non_dominated_sort::*;

mod nsga2;
pub use self::nsga2::select_and_rank;
