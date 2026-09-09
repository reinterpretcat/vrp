#[cfg(test)]
#[path = "../../../tests/unit/solver/search/infeasible_search_test.rs"]
mod infeasible_search_test;

use crate::construction::heuristics::*;
use crate::construction::probing::repair_solution_from_unknown;
use crate::models::*;
use crate::solver::*;
use std::cmp::Ordering;
use std::sync::Arc;

// A narrow normalized band lets the first move cross the feasibility boundary without making
// deeply infeasible solutions attractive. Retained lineages contract back towards feasibility.
const INITIAL_INFEASIBILITY_TOLERANCE: Float = 0.05;
const INFEASIBILITY_TOLERANCE_CONTRACTION: Float = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RelaxedSearchStep {
    Restore,
    Seed,
    Repair,
}

/// Advances a persistent relaxed lineage using only constraints which explicitly support relaxation.
///
/// A first visit seeds Rosomaxa's hidden relaxed archive. Later visits search from a retained relaxed parent and
/// attempt to rebuild it under the original constraints; unsuccessful repair returns the advanced relaxed descendant.
pub struct InfeasibleSearch {
    inner_search: TargetSearchOperator,
    recovery_operator: Arc<dyn Recreate>,
    max_repeat_count: usize,
    alternative_objectives_probability: (Float, Float),
}

impl InfeasibleSearch {
    /// Creates a new instance of `InfeasibleSearch`.
    pub fn new(
        inner_search: TargetSearchOperator,
        recovery_operator: Arc<dyn Recreate>,
        max_repeat_count: usize,
        alternative_objectives_probability: (Float, Float),
    ) -> Self {
        assert!(max_repeat_count > 0, "repeat count must be positive");
        assert!(
            (0. ..=alternative_objectives_probability.1).contains(&alternative_objectives_probability.0)
                && alternative_objectives_probability.1 <= 1.,
            "objective probability range must be within [0, 1]"
        );

        Self { inner_search, recovery_operator, max_repeat_count, alternative_objectives_probability }
    }

    fn recover_individual(
        &self,
        orig_refinement_ctx: &RefinementContext,
        new_insertion_ctx: InsertionContext,
    ) -> InsertionContext {
        let new_insertion_ctx = repair_solution_from_unknown(&new_insertion_ctx, &|| {
            InsertionContext::new(orig_refinement_ctx.problem.clone(), orig_refinement_ctx.environment.clone())
        });

        // NOTE: give a chance to rearrange unassigned jobs
        let mut new_insertion_ctx = self.recovery_operator.run(orig_refinement_ctx, new_insertion_ctx);
        finalize_insertion_ctx(&mut new_insertion_ctx);

        new_insertion_ctx
    }
}

impl HeuristicSearchOperator for InfeasibleSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        let refinement_ctx = heuristic_ctx;
        // Revisit a structurally distant relaxed resident when possible. Otherwise, seed the hidden archive from
        // the regular parent selected by Rosomaxa.
        let relaxed_parent = refinement_ctx.selected_relaxed(solution);
        let repeat_count = refinement_ctx.environment.random.uniform_int(1, self.max_repeat_count as i32) as usize;

        // First exhaust the same education under strict constraints. Crossing the feasibility boundary is useful
        // only when the local neighborhoods cannot make immediate progress from the selected regular parent.
        if relaxed_parent.is_none() {
            let mut candidate = self.inner_search.search(refinement_ctx, solution);
            finalize_insertion_ctx(&mut candidate);

            if is_improvement(refinement_ctx, &candidate, solution) {
                return candidate;
            }
        }

        let Some(new_insertion_ctx) =
            create_relaxed_insertion_ctx(relaxed_parent.unwrap_or(solution), self.alternative_objectives_probability)
        else {
            return solution.deep_copy();
        };
        let mut new_refinement_ctx = create_relaxed_refinement_ctx(&new_insertion_ctx);

        // Keep a retained lineage monotone: if none of its descendants is better under the tighter
        // violation band, the relaxed parent remains available instead of being forcibly replaced.
        if relaxed_parent.is_some() {
            new_refinement_ctx.add_solution(new_insertion_ctx.deep_copy());
        }

        let repeat_count = if relaxed_parent.is_some() { repeat_count } else { repeat_count.saturating_sub(1).max(1) };

        let mut initial = Some(new_insertion_ctx);
        for _ in 0..repeat_count {
            // From diversity reasons, do not put the original solution into the temporary population.
            let new_insertion_ctx = match initial.take() {
                Some(initial) => self.inner_search.search(&new_refinement_ctx, &initial),
                _ => self.inner_search.search(&new_refinement_ctx, get_random_individual(&new_refinement_ctx)),
            };

            // Exact violation degrees are derived only after every route and solution state has been refreshed.
            let mut new_insertion_ctx = new_insertion_ctx;
            finalize_insertion_ctx(&mut new_insertion_ctx);
            new_refinement_ctx.add_solution(new_insertion_ctx);
        }

        let objective = refinement_ctx.objective();
        let improved_feasible = new_refinement_ctx
            .ranked()
            .filter(|solution| get_violation(solution) == Some(0.))
            .min_by(|left, right| objective.total_order(left, right))
            .filter(|candidate| is_improvement(refinement_ctx, candidate, solution))
            .map(|candidate| candidate.deep_copy());

        // A useful feasible descendant should not be hidden merely because an objective-better infeasible sibling
        // ranks first under the relaxed goal. Keep the relaxed lineage only when no immediate progress was found.
        if let Some(candidate) = improved_feasible {
            return restore_feasible_individual(refinement_ctx, candidate);
        }

        let Some(relaxed) = new_refinement_ctx.ranked().next().map(|solution| solution.deep_copy()) else {
            return solution.deep_copy();
        };
        let candidate_violation = get_violation(&relaxed).unwrap_or(Float::MAX);
        let next_step = get_next_step(relaxed_parent.is_some(), candidate_violation);

        match next_step {
            RelaxedSearchStep::Restore => restore_feasible_individual(refinement_ctx, relaxed),
            RelaxedSearchStep::Seed => relaxed,
            RelaxedSearchStep::Repair => {
                let recovered = self.recover_individual(refinement_ctx, relaxed.deep_copy());
                let parent_order = objective.total_order(&recovered, solution);
                let best_order = refinement_ctx.ranked().next().map(|best| objective.total_order(&recovered, best));
                let is_improvement = is_repair_improvement(refinement_ctx.selection_phase(), parent_order, best_order);

                if is_improvement {
                    recovered
                } else {
                    // Keep advancing the relaxed resident after a failed repair instead of restarting its basin.
                    relaxed
                }
            }
        }
    }
}

fn is_repair_improvement(phase: SelectionPhase, parent_order: Ordering, best_order: Option<Ordering>) -> bool {
    match phase {
        SelectionPhase::Exploration => parent_order == Ordering::Less,
        SelectionPhase::Initial | SelectionPhase::Exploitation => best_order == Some(Ordering::Less),
    }
}

fn is_improvement(refinement_ctx: &RefinementContext, candidate: &InsertionContext, parent: &InsertionContext) -> bool {
    let objective = refinement_ctx.objective();
    let parent_order = objective.total_order(candidate, parent);
    let best_order = refinement_ctx.ranked().next().map(|best| objective.total_order(candidate, best));

    is_repair_improvement(refinement_ctx.selection_phase(), parent_order, best_order)
}

fn get_next_step(has_relaxed_parent: bool, candidate_violation: Float) -> RelaxedSearchStep {
    if candidate_violation == 0. {
        RelaxedSearchStep::Restore
    } else if has_relaxed_parent {
        RelaxedSearchStep::Repair
    } else {
        RelaxedSearchStep::Seed
    }
}

fn get_violation(insertion_ctx: &InsertionContext) -> Option<Float> {
    insertion_ctx.solution.state.get_relaxed_violation().copied()
}

fn get_infeasibility_tolerance(violation: Option<Float>) -> Float {
    violation.map_or(INITIAL_INFEASIBILITY_TOLERANCE, |violation| violation * INFEASIBILITY_TOLERANCE_CONTRACTION)
}

fn restore_feasible_individual(
    refinement_ctx: &RefinementContext,
    mut insertion_ctx: InsertionContext,
) -> InsertionContext {
    insertion_ctx.problem = refinement_ctx.problem.clone();
    finalize_insertion_ctx(&mut insertion_ctx);

    insertion_ctx
}

fn create_relaxed_refinement_ctx(new_insertion_ctx: &InsertionContext) -> RefinementContext {
    let problem = new_insertion_ctx.problem.clone();
    let environment = new_insertion_ctx.environment.clone();
    let population = Box::new(ElitismPopulation::new(problem.goal.clone(), environment.random.clone(), 4, 4));

    // NOTE statistic is reset to default
    RefinementContext::new(problem, population, TelemetryMode::None, environment)
}

fn create_relaxed_insertion_ctx(
    insertion_ctx: &InsertionContext,
    alt_objectives_probability: (Float, Float),
) -> Option<InsertionContext> {
    let problem = &insertion_ctx.problem;
    let random = &insertion_ctx.environment.random;

    let alternative_probability = random.uniform_real(alt_objectives_probability.0, alt_objectives_probability.1);
    let alternative = if !problem.goal.is_relaxed() && random.is_hit(alternative_probability) {
        problem.goal.get_random_alternative(random.as_ref()).unwrap_or_else(|| problem.goal.as_ref().clone())
    } else {
        problem.goal.as_ref().clone()
    };
    let infeasibility_tolerance = get_infeasibility_tolerance(get_violation(insertion_ctx));
    let variant = Arc::new(alternative.relaxed(infeasibility_tolerance)?);

    let mut insertion_ctx = insertion_ctx.deep_copy();
    insertion_ctx.problem = Arc::new(Problem {
        fleet: problem.fleet.clone(),
        jobs: problem.jobs.clone(),
        locks: problem.locks.clone(),
        goal: variant,
        activity: problem.activity.clone(),
        transport: problem.transport.clone(),
        extras: problem.extras.clone(),
    });

    finalize_insertion_ctx(&mut insertion_ctx);

    Some(insertion_ctx)
}

fn get_random_individual(new_refinement_ctx: &RefinementContext) -> &InsertionContext {
    let selected = new_refinement_ctx.selected().collect::<Vec<_>>();
    let skip = new_refinement_ctx.environment.random.uniform_int(0, selected.len() as i32 - 1) as usize;

    selected.get(skip).expect("no individual")
}
