#[cfg(test)]
#[path = "../../../tests/unit/solver/population/greedy_test.rs"]
mod greedy_test;

use crate::algorithms::nsga2::Objective;
use crate::models::Problem;
use crate::solver::population::{Individual, SelectionPhase};
use crate::solver::{Population, Statistics};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::iter::{empty, repeat};
use std::sync::Arc;

/// A population which keeps track of the best known individuals only.
/// If solutions are equal, prefers to keep first discovered.
pub struct Greedy {
    problem: Arc<Problem>,
    selection_size: usize,
    best_known: Option<Individual>,
}

impl Population for Greedy {
    fn add_all(&mut self, individuals: Vec<Individual>) -> bool {
        #[allow(clippy::unnecessary_fold)]
        individuals.into_iter().fold(false, |acc, individual| acc || self.add(individual))
    }

    fn add(&mut self, individual: Individual) -> bool {
        if let Some(best_known) = &self.best_known {
            if self.problem.objective.total_order(best_known, &individual) != Ordering::Greater {
                return false;
            }
        }

        self.best_known = Some(individual);

        true
    }

    fn on_generation(&mut self, _: &Statistics) {}

    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering {
        self.problem.objective.total_order(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        if let Some(best_known) = self.best_known.as_ref() {
            Box::new(repeat(best_known).take(self.selection_size))
        } else {
            Box::new(empty())
        }
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Individual, usize)> + 'a> {
        Box::new(self.best_known.iter().map(|individual| (individual, 0)))
    }

    fn size(&self) -> usize {
        if self.best_known.is_some() {
            1
        } else {
            0
        }
    }

    fn selection_phase(&self) -> SelectionPhase {
        SelectionPhase::Exploitation
    }
}

impl Display for Greedy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let values = if let Some(best_known) = &self.best_known {
            best_known.get_fitness_values().map(|v| format!("{:.7}", v)).collect::<Vec<_>>().join(",")
        } else {
            "".to_string()
        };

        write!(f, "[{}],", values)
    }
}

impl Greedy {
    /// Creates a new instance of `Greedy`.
    pub fn new(problem: Arc<Problem>, selection_size: usize, best_known: Option<Individual>) -> Self {
        Self { problem, selection_size, best_known }
    }
}
