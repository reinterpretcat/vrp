//! This module contains a hyper-heuristic logic.

mod dynamic_selective;
pub use self::dynamic_selective::*;

mod static_selective;
pub use self::static_selective::*;

use crate::prelude::*;
use std::fmt::Display;
use std::marker::PhantomData;

/// A heuristic operator which is responsible to change passed solution.
pub trait HeuristicOperator {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Performs search for a new (better) solution using given one.
    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution;
}

/// Represents a hyper heuristic functionality.
pub trait HyperHeuristic: Display {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Performs a new search in the solution space using selected solutions.
    fn search(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution>;
}
