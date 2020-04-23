#[cfg(test)]
#[path = "../../../tests/unit/refinement/population/objective_test.rs"]
mod objective_test;

use super::DominanceOrd;
use std::cmp::Ordering;
use std::marker::PhantomData;

/// An *objective* defines a *total ordering relation* by `dominance_ord` method from `DominanceOrd`
/// trait and a *distance metric* on a `solutions` set of type `T`. Given any two solutions, an
/// objective answers the following two questions:
///
/// - "which solution is the better one" (total order)
///
/// - "how similar are the two solutions" (distance metric)
///
pub trait Objective: DominanceOrd {
    /// The output type of the distance metric.
    type Distance: Sized;

    /// Gets a distance metric between any two solution values.
    ///
    /// The distance metric answer the question, how similar the solutions `a` and `b` are,
    /// according to the objective.  A zero value would mean, that both solutions are in fact the same,
    /// according to the objective. Larger magnitudes would mean "less similar".
    ///
    /// Note: Distance values can be negative, i.e. the caller is responsible for obtaining absolute values.
    fn distance(&self, a: &Self::T, b: &Self::T) -> Self::Distance;
}

/// A hierarchy objective which separates multiple objectives into two groups: primary and secondary.
/// An objective from primary group is considered more important than secondary one.
pub struct HierarchyObjective<'a, S, D>
where
    S: 'a,
    D: 'a,
{
    primary: MultiObjective<'a, S, D>,
    secondary: MultiObjective<'a, S, D>,
    _solution: PhantomData<S>,
    _distance: PhantomData<D>,
}

impl<'a, S, D> HierarchyObjective<'a, S, D>
where
    S: 'a,
    D: 'a,
{
    pub fn new(primary: MultiObjective<'a, S, D>, secondary: MultiObjective<'a, S, D>) -> Self {
        Self { primary, secondary, _solution: PhantomData, _distance: PhantomData }
    }
}

impl<'a, S, D> DominanceOrd for HierarchyObjective<'a, S, D>
where
    S: 'a,
    D: 'a,
{
    type T = S;

    fn dominance_ord(&self, a: &Self::T, b: &Self::T) -> Ordering {
        match self.primary.dominance_ord(a, b) {
            Ordering::Equal => self.secondary.dominance_ord(a, b),
            order @ _ => order,
        }
    }
}

/// A multi objective which combines multiple objectives and allows to compare solutions based on
/// dominance ordering. All objectives are considered as equally important.
pub struct MultiObjective<'a, S, D>
where
    S: 'a,
    D: 'a,
{
    pub objectives: &'a [&'a dyn Objective<T = S, Distance = D>],
    _solution: PhantomData<S>,
    _distance: PhantomData<D>,
}

impl<'a, S, D> MultiObjective<'a, S, D>
where
    S: 'a,
    D: 'a,
{
    pub fn new(objectives: &'a [&'a dyn Objective<T = S, Distance = D>]) -> Self {
        Self { objectives, _solution: PhantomData, _distance: PhantomData }
    }
}

impl<'a, S, D> DominanceOrd for MultiObjective<'a, S, D>
where
    S: 'a,
    D: 'a,
{
    type T = S;

    fn dominance_ord(&self, a: &Self::T, b: &Self::T) -> Ordering {
        let mut less_cnt = 0;
        let mut greater_cnt = 0;

        for objective in self.objectives.iter() {
            match objective.dominance_ord(a, b) {
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
}
