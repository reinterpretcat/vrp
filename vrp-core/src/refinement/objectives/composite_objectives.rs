#[cfg(test)]
#[path = "../../../tests/unit/refinement/objectives/composite_objectives_test.rs"]
mod composite_objectives_test;

use crate::refinement::objectives::Objective;
use std::cmp::Ordering;
use std::marker::PhantomData;

/// A hierarchy objective which separates multiple objectives into two groups: primary and secondary.
/// An objective from primary group is considered more important than secondary one.
pub struct HierarchyObjective<S> {
    primary: MultiObjective<S>,
    secondary: MultiObjective<S>,
    _solution: PhantomData<S>,
}

impl<S> HierarchyObjective<S>
where
    S: Send + Sync,
{
    pub fn new(primary: MultiObjective<S>, secondary: MultiObjective<S>) -> Self {
        Self { primary, secondary, _solution: PhantomData }
    }
}

impl<S> Objective for HierarchyObjective<S> {
    type Solution = S;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        match self.primary.total_order(a, b) {
            Ordering::Equal => self.secondary.total_order(a, b),
            order @ _ => order,
        }
    }

    fn distance(&self, _a: &Self::Solution, _b: &Self::Solution) -> f64 {
        unimplemented!()
    }

    fn fitness(&self, _solution: &Self::Solution) -> f64 {
        unimplemented!()
    }
}

/// A multi objective which combines multiple objectives and allows to compare solutions based on
/// dominance ordering. All objectives are considered as equally important.
pub struct MultiObjective<S> {
    pub objectives: Vec<Box<dyn Objective<Solution = S> + Send + Sync>>,
    _solution: PhantomData<S>,
}

impl<S> Default for MultiObjective<S> {
    fn default() -> Self {
        unimplemented!()
    }
}

impl<S> MultiObjective<S> {
    pub fn new(objectives: Vec<Box<dyn Objective<Solution = S> + Send + Sync>>) -> Self {
        Self { objectives, _solution: PhantomData }
    }
}

impl<S> Objective for MultiObjective<S> {
    type Solution = S;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        let mut less_cnt = 0;
        let mut greater_cnt = 0;

        for objective in self.objectives.iter() {
            match objective.total_order(a, b) {
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

    fn distance(&self, _a: &Self::Solution, _b: &Self::Solution) -> f64 {
        unimplemented!()
    }

    fn fitness(&self, _solution: &Self::Solution) -> f64 {
        unimplemented!()
    }
}
