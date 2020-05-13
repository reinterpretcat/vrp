//! This module contains a logic for processing multiple solutions and multi objective optimization
//! based on `Non Dominated Sorting Genetic Algorithm II` algorithm.
//!
//! A Non Dominated Sorting Genetic Algorithm II (NSGA-II) is a popular multi objective optimization
//! algorithm with three special characteristics:
//!
//! - fast non-dominated sorting approach
//! - fast crowded distance estimation procedure
//! - simple crowded comparison operator
//!
//! For more details regarding NSGA-II algorithm details, check original paper "A fast and elitist
//! multiobjective genetic algorithm: NSGA-II", Kalyanmoy Deb et al. DOI: `0.1109/4235.996017`
//!
//! A NSGA-II implementation in this module is based on the source code from the following repositories:
//!
//! - [dominance order trait](https://github.com/mneumann/dominance-ord-rs)
//! - [fast non-dominated sort algorithm](https://github.com/mneumann/non-dominated-sort-rs)
//! - [NSGA-II implementation](https://github.com/mneumann/nsga2-rs)
//!
//! which are released under MIT License (MIT), copyright (c) 2016 Michael Neumann
//!

mod crowding_distance;
use self::crowding_distance::*;

mod non_dominated_sort;
use self::non_dominated_sort::*;

mod nsga2;
pub use self::nsga2::select_and_rank;

mod objective;
pub use self::objective::*;
