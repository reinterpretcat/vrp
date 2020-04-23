//! A multi objective algorithms based on NSGA-2.
//!
//! The code is based on the source code from the following repos:
//!
//! https://github.com/mneumann/dominance-ord-rs
//! https://github.com/mneumann/non-dominated-sort-rs
//! https://github.com/mneumann/nsga2-rs
//!
//! The MIT License (MIT)
//! Copyright (c) 2016 Michael Neumann
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
