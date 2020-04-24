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

mod crowding_distance;
pub use self::crowding_distance::*;

mod dominance_ord;
pub use self::dominance_ord::DominanceOrd;

mod non_dominated_sort;
pub use self::non_dominated_sort::*;

mod objective;
pub use self::objective::*;

mod selection;
pub use self::selection::select_and_rank;
