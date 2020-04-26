//! Implementation of the [Fast Non-Dominated Sort Algorithm][1] as used by NSGA-II.
//! Time complexity is `O(K * N^2)`, where `K` is the number of objectives and `N` the number of solutions.
//!
//! Non-dominated sorting is used in multi-objective (multivariate) optimization to group solutions
//! into non-dominated Pareto fronts according to their objectives. In the existence of multiple
//! objectives, a solution can happen to be better in one objective while at the same time worse in
//! another objective, and as such none of the two solutions _dominates_ the other.
//!
//! [1]: https://www.iitk.ac.in/kangal/Deb_NSGA-II.pdf "A Fast and Elitist Multiobjective Genetic Algorithm: NSGA-II)"

#[cfg(test)]
#[path = "../../../tests/unit/solver/population/non_dominated_sort_test.rs"]
mod non_dominated_sort_test;

use crate::models::common::Objective;
use std::cmp::Ordering;

type SolutionIdx = usize;

#[derive(Debug, Clone)]
pub struct Front<'s, S: 's> {
    dominated_solutions: Vec<Vec<SolutionIdx>>,
    domination_count: Vec<usize>,
    previous_front: Vec<SolutionIdx>,
    current_front: Vec<SolutionIdx>,
    rank: usize,
    solutions: &'s [S],
}

impl<'f, 's: 'f, S: 's> Front<'s, S> {
    pub fn rank(&self) -> usize {
        self.rank
    }

    /// Iterates over the elements of the front.
    pub fn iter(&'f self) -> FrontElemIter<'f, 's, S> {
        FrontElemIter { front: self, next_idx: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.current_front.is_empty()
    }

    pub fn next_front(self) -> Self {
        let Front { dominated_solutions, mut domination_count, previous_front, current_front, rank, solutions } = self;

        // reuse the previous_front
        let mut next_front = previous_front;
        next_front.clear();

        for &p_i in current_front.iter() {
            for &q_i in dominated_solutions[p_i].iter() {
                debug_assert!(domination_count[q_i] > 0);
                domination_count[q_i] -= 1;
                if domination_count[q_i] == 0 {
                    // q_i is not dominated by any other solution. it belongs to the next front.
                    next_front.push(q_i);
                }
            }
        }

        Self {
            dominated_solutions,
            domination_count,
            previous_front: current_front,
            current_front: next_front,
            rank: rank + 1,
            solutions,
        }
    }
}

pub struct FrontElemIter<'f, 's: 'f, S: 's> {
    front: &'f Front<'s, S>,
    next_idx: SolutionIdx,
}

impl<'f, 's: 'f, S: 's> Iterator for FrontElemIter<'f, 's, S> {
    type Item = (&'s S, usize);

    fn next(&mut self) -> Option<Self::Item> {
        match self.front.current_front.get(self.next_idx) {
            Some(&solution_idx) => {
                self.next_idx += 1;
                Some((&self.front.solutions[solution_idx], solution_idx))
            }
            None => None,
        }
    }
}

/// Performs a non-dominated sort of `solutions`. Returns the first Pareto front.
pub fn non_dominated_sort<'s, S, O>(solutions: &'s [S], objective: &O) -> Front<'s, S>
where
    O: Objective<Solution = S>,
{
    // the indices of the solutions that are dominated by this `solution`
    let mut dominated_solutions: Vec<Vec<SolutionIdx>> = solutions.iter().map(|_| Vec::new()).collect();

    // for each solutions, we keep a domination count, i.e. the number of solutions that dominate the solution
    let mut domination_count: Vec<usize> = solutions.iter().map(|_| 0).collect();

    let mut current_front: Vec<SolutionIdx> = Vec::new();

    // initial pass over each combination: O(n*n / 2)
    let mut iter = solutions.iter().enumerate();
    while let Some((p_i, p)) = iter.next() {
        let mut pair_iter = iter.clone();
        while let Some((q_i, q)) = pair_iter.next() {
            match objective.total_order(p, q) {
                Ordering::Less => {
                    // p dominates q, add `q` to the set of solutions dominated by `p`
                    dominated_solutions[p_i].push(q_i);
                    // q is dominated by p
                    domination_count[q_i] += 1;
                }
                Ordering::Greater => {
                    // p is dominated by q, add `p` to the set of solutions dominated by `q`
                    dominated_solutions[q_i].push(p_i);
                    // q dominates p, increment domination counter of `p`
                    domination_count[p_i] += 1
                }
                Ordering::Equal => {}
            }
        }
        // if domination_count drops to zero, push index to front
        if domination_count[p_i] == 0 {
            current_front.push(p_i);
        }
    }

    Front { dominated_solutions, domination_count, previous_front: Vec::new(), current_front, rank: 0, solutions }
}
