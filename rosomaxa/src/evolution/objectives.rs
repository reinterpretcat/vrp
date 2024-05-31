//! Specifies objective functions.

use std::cmp::Ordering;

/// An *objective* function defines a *total ordering relation* and a *fitness metrics* on a set of
/// `solutions`. Given any two solutions, an objective answers the following two questions:
///
/// - "which solution is the better one" (total order)
/// - "how are two solutions close to each other" (fitness vector metrics)
pub trait HeuristicObjective: Send + Sync {
    /// The solution value type that we define the objective on.
    type Solution;

    /// An objective defines a total ordering between any two solution values.
    ///
    /// This answers the question, is solution `a` better, equal or worse than solution `b`,
    /// according to the objective.
    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering;

    /// An objective fitness values for given `solution`.
    fn fitness<'a>(&'a self, solution: &'a Self::Solution) -> Box<dyn Iterator<Item = f64> + 'a>;
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
