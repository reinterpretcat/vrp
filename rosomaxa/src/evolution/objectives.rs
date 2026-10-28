//! Specifies objective functions.

use std::cmp::Ordering;

/// A *heuristic objective* function defines a *total ordering relation* between any two solutions
/// as a goal of optimization.
pub trait HeuristicObjective: Send + Sync {
    /// The solution value type that we define the objective on.
    type Solution;

    /// An objective defines a total ordering between any two solution values.
    ///
    /// This answers the question, is solution `a` better, equal or worse than solution `b` according
    /// to the goal of optimization.
    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering;
}
