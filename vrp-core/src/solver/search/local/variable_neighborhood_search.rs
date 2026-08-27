#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/variable_neighborhood_search_test.rs"]
mod variable_neighborhood_search_test;

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;
use crate::solver::search::LocalOperator;
use rosomaxa::prelude::{HeuristicContext, HeuristicObjective, HeuristicStatistics};
use std::cmp::Ordering;
use std::sync::Arc;

// A broad neighborhood is useful before stagnation too, but it should not dominate cheap VND passes.
const EXTENDED_INTERVAL: usize = 10;
const STAGNATION_EXTENDED_INTERVAL: usize = 2;
const STAGNATION_WINDOW: usize = 1000;

/// Combines complementary local operators into a randomized variable-neighborhood descent.
///
/// Only strict improvements of the configured objective are accepted. After an improvement, every
/// neighborhood becomes eligible again because the move can expose opportunities which previously
/// did not exist. The search stops after a complete pass without improvement or after the accepted
/// move budget is exhausted. A configured extended neighborhood is tried periodically, at most once
/// per descent, and becomes more frequent during stagnation.
pub struct VariableNeighborhoodSearch {
    operators: Vec<Arc<dyn LocalOperator>>,
    extended_operator: Option<Arc<dyn LocalOperator>>,
    max_improvements: usize,
}

impl VariableNeighborhoodSearch {
    /// Creates a variable-neighborhood search from improvement and proposal operators.
    pub fn new(operators: Vec<Arc<dyn LocalOperator>>, max_improvements: usize) -> Self {
        assert!(!operators.is_empty());
        assert!(max_improvements > 0);

        Self { operators, extended_operator: None, max_improvements }
    }

    /// Adds a broad neighborhood which is sampled periodically and more often during stagnation.
    pub fn with_extended_operator(mut self, operator: Arc<dyn LocalOperator>) -> Self {
        self.extended_operator = Some(operator);
        self
    }
}

impl LocalOperator for VariableNeighborhoodSearch {
    fn explore(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        let random = insertion_ctx.environment.random.as_ref();
        let mut current = None;
        let use_extended =
            self.extended_operator.is_some() && should_use_extended_operator(refinement_ctx.statistics());
        let operator_count = self.operators.len() + usize::from(use_extended);
        let mut remaining = (0..operator_count).collect::<Vec<_>>();
        let mut improvements = 0;
        let mut extended_attempted = false;

        while !remaining.is_empty() && improvements < self.max_improvements {
            if insertion_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached()) {
                break;
            }

            let index = random.uniform_int(0, remaining.len() as i32 - 1) as usize;
            let operator_idx = remaining.swap_remove(index);
            let operator = if operator_idx < self.operators.len() {
                &self.operators[operator_idx]
            } else {
                extended_attempted = true;
                self.extended_operator.as_ref().expect("extended operator is not configured")
            };
            let source = current.as_ref().unwrap_or(insertion_ctx);

            let Some(candidate) = operator.explore(refinement_ctx, source) else {
                continue;
            };

            if insertion_ctx.problem.goal.total_order(&candidate, source) == Ordering::Less {
                current = Some(candidate);
                improvements += 1;
                remaining.clear();
                remaining.extend(0..self.operators.len());
                if use_extended && !extended_attempted {
                    remaining.push(self.operators.len());
                }
            }
        }

        current
    }
}

fn should_use_extended_operator(statistics: &HeuristicStatistics) -> bool {
    let is_stagnating = statistics.generation >= STAGNATION_WINDOW && statistics.improvement_1000_ratio == 0.;
    let interval = if is_stagnating { STAGNATION_EXTENDED_INTERVAL } else { EXTENDED_INTERVAL };

    statistics.generation.is_multiple_of(interval)
}
