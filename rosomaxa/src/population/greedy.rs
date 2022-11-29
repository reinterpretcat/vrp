#[cfg(test)]
#[path = "../../tests/unit/population/greedy_test.rs"]
mod greedy_test;

use super::*;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::iter::{empty, repeat};
use std::sync::Arc;

/// A population which keeps track of the best known individuals only.
/// If solutions are equal, prefers to keep first discovered.
pub struct Greedy<O, S>
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    objective: Arc<O>,
    selection_size: usize,
    best_known: Option<S>,
}

impl<O, S> HeuristicPopulation for Greedy<O, S>
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    type Objective = O;
    type Individual = S;

    fn add_all(&mut self, individuals: Vec<Self::Individual>) -> bool {
        #[allow(clippy::unnecessary_fold)]
        individuals.into_iter().fold(false, |acc, individual| acc || self.add(individual))
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        if let Some(best_known) = &self.best_known {
            if self.objective.total_order(best_known, &individual) != Ordering::Greater {
                return false;
            }
        }

        self.best_known = Some(individual);

        true
    }

    fn on_generation(&mut self, _: &HeuristicStatistics) {}

    fn cmp(&self, a: &Self::Individual, b: &Self::Individual) -> Ordering {
        self.objective.total_order(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        if let Some(best_known) = self.best_known.as_ref() {
            Box::new(repeat(best_known).take(self.selection_size))
        } else {
            Box::new(empty())
        }
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Self::Individual, usize)> + 'a> {
        Box::new(self.best_known.iter().map(|individual| (individual, 0)))
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        Box::new(self.best_known.iter())
    }

    fn size(&self) -> usize {
        usize::from(self.best_known.is_some())
    }

    fn selection_phase(&self) -> SelectionPhase {
        SelectionPhase::Exploitation
    }
}

impl<O, S> Display for Greedy<O, S>
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let values = if let Some(best_known) = &self.best_known {
            best_known.fitness().map(|v| format!("{:.7}", v)).collect::<Vec<_>>().join(",")
        } else {
            "".to_string()
        };

        write!(f, "[{}]", values)
    }
}

impl<O, S> Greedy<O, S>
where
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `Greedy`.
    pub fn new(objective: Arc<O>, selection_size: usize, best_known: Option<S>) -> Self {
        Self { objective, selection_size, best_known }
    }
}
