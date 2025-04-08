#[cfg(test)]
#[path = "../../tests/unit/population/greedy_test.rs"]
mod greedy_test;

use super::*;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::iter::empty;
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

    fn select(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        match self.best_known.as_ref() {
            Some(best_known) => Box::new(std::iter::repeat_n(best_known, self.selection_size)),
            _ => Box::new(empty()),
        }
    }

    fn ranked(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        Box::new(self.best_known.iter())
    }

    fn all(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
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
        let values = match &self.best_known {
            Some(best_known) => best_known.fitness().map(|v| format!("{v:.7}")).collect::<Vec<_>>().join(","),
            _ => "".to_string(),
        };

        write!(f, "[{values}]")
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
