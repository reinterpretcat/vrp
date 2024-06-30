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

/// Calculates dominance order of two solutions using ordering functions.
pub fn dominance_order<'a, T: 'a, Order, Iter>(a: &'a T, b: &'a T, ordering_fns: Iter) -> Ordering
where
    Order: Fn(&'a T, &'a T) -> Ordering,
    Iter: Iterator<Item = Order>,
{
    let mut less_cnt = 0;
    let mut greater_cnt = 0;

    for ordering_fn in ordering_fns {
        match ordering_fn(a, b) {
            Ordering::Less => {
                less_cnt += 1;
            }
            Ordering::Greater => {
                greater_cnt += 1;
            }
            Ordering::Equal => {}
        }
    }

    if less_cnt > 0 && greater_cnt == 0 {
        Ordering::Less
    } else if greater_cnt > 0 && less_cnt == 0 {
        Ordering::Greater
    } else {
        debug_assert!((less_cnt > 0 && greater_cnt > 0) || (less_cnt == 0 && greater_cnt == 0));
        Ordering::Equal
    }
}
