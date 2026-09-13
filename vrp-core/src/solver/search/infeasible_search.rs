#[cfg(test)]
#[path = "../../../tests/unit/solver/search/infeasible_search_test.rs"]
mod infeasible_search_test;

use crate::construction::heuristics::*;
use crate::construction::probing::repair_solution_from_unknown;
use crate::models::*;
use crate::solver::search::{LocalOperator, VariableNeighborhoodResult, VariableNeighborhoodSearch};
use crate::solver::*;
use rosomaxa::hyper::HeuristicEscapeOperator;
use rosomaxa::population::RelaxedSolutionProgress;
use std::cmp::Ordering;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

// A narrow normalized band lets the first move cross the feasibility boundary without making
// deeply infeasible solutions attractive. Retained lineages contract back towards feasibility.
const INITIAL_INFEASIBILITY_TOLERANCE: Float = 0.05;
const INFEASIBILITY_TOLERANCE_CONTRACTION: Float = 0.5;
// Escape calls are sparse, so an episode gets several visits, but its hidden trajectory cannot live forever.
const RELAXED_CHECKPOINT_VISITS: usize = 16;

#[derive(Clone, Copy)]
pub(crate) struct RelaxedSearchState {
    progress: RelaxedSolutionProgress,
    remaining_visits: usize,
    recovered_checkpoint: Option<usize>,
}

impl RelaxedSearchState {
    fn new(episode: usize, needs_recovery: bool) -> Self {
        Self {
            progress: RelaxedSolutionProgress::new(episode, 0),
            remaining_visits: RELAXED_CHECKPOINT_VISITS,
            recovered_checkpoint: (!needs_recovery).then_some(0),
        }
    }

    fn advance(&mut self) {
        self.progress = RelaxedSolutionProgress::new(self.progress.episode(), self.progress.step() + 1);
    }

    fn complete_visit(&mut self) {
        self.remaining_visits = self.remaining_visits.saturating_sub(1);
    }

    fn needs_recovery(self) -> bool {
        self.recovered_checkpoint != Some(self.progress.step())
    }

    fn mark_recovered(&mut self) {
        self.recovered_checkpoint = Some(self.progress.step());
    }
}

custom_solution_state!(pub(crate) RelaxedSearch typeof RelaxedSearchState);

pub(crate) fn get_relaxed_solution_progress(insertion_ctx: &InsertionContext) -> Option<RelaxedSolutionProgress> {
    insertion_ctx.solution.state.get_relaxed_violation()?;

    insertion_ctx.solution.state.get_relaxed_search().map(|state| state.progress)
}

pub(crate) fn is_relaxed_solution_continuation(insertion_ctx: &InsertionContext) -> bool {
    get_violation(insertion_ctx).is_some()
        && insertion_ctx.solution.state.get_relaxed_search().is_some_and(|state| state.remaining_visits > 0)
}

pub(crate) fn end_relaxed_solution_continuation(insertion_ctx: &mut InsertionContext) {
    if let Some(mut state) = insertion_ctx.solution.state.get_relaxed_search().copied() {
        state.remaining_visits = 0;
        insertion_ctx.solution.state.set_relaxed_search(state);
    }
}

fn is_relaxed_parent_eligible(insertion_ctx: &InsertionContext) -> bool {
    insertion_ctx.solution.state.get_relaxed_search().is_none_or(|state| state.remaining_visits > 0)
}

/// Advances a persistent relaxed lineage using only constraints which explicitly support relaxation.
///
/// A first visit seeds Rosomaxa's hidden relaxed archive. Later visits search from a retained relaxed parent and
/// attempt to rebuild it under the original constraints. Reaching feasibility does not end the hidden trajectory: an
/// independent strict child can be returned while the relaxed copy remains available for another boundary crossing.
/// Checkpoint progress and its finite lease travel with the solution instead of being tied to a mutable operator or
/// GSOM coordinate.
pub struct InfeasibleSearch {
    inner_search: InfeasibleEducation,
    recovery_operator: Arc<dyn Recreate>,
    max_repeat_count: usize,
    alternative_objectives_probability: (Float, Float),
    next_episode: AtomicUsize,
}

enum InfeasibleEducation {
    VariableNeighborhood(Arc<VariableNeighborhoodSearch>),
    #[cfg(test)]
    Mock(TargetSearchOperator),
}

impl InfeasibleEducation {
    fn search(&self, refinement_ctx: &RefinementContext, insertion_ctx: &InsertionContext) -> InsertionContext {
        match self {
            Self::VariableNeighborhood(search) => {
                search.explore(refinement_ctx, insertion_ctx).unwrap_or_else(|| insertion_ctx.deep_copy())
            }
            #[cfg(test)]
            Self::Mock(search) => search.search(refinement_ctx, insertion_ctx),
        }
    }

    fn search_relaxed(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
        objective: &GoalContext,
    ) -> VariableNeighborhoodResult {
        match self {
            Self::VariableNeighborhood(search) => search.explore_relaxed(refinement_ctx, insertion_ctx, objective),
            #[cfg(test)]
            Self::Mock(search) => VariableNeighborhoodResult {
                endpoint: Some(search.search(refinement_ctx, insertion_ctx)),
                feasible_intermediate: None,
            },
        }
    }
}

impl InfeasibleSearch {
    /// Creates infeasible search with bounded variable-neighborhood education.
    pub fn new(
        inner_search: Arc<VariableNeighborhoodSearch>,
        recovery_operator: Arc<dyn Recreate>,
        max_repeat_count: usize,
        alternative_objectives_probability: (Float, Float),
    ) -> Self {
        Self::create(
            InfeasibleEducation::VariableNeighborhood(inner_search),
            recovery_operator,
            max_repeat_count,
            alternative_objectives_probability,
        )
    }

    fn create(
        inner_search: InfeasibleEducation,
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

        Self {
            inner_search,
            recovery_operator,
            max_repeat_count,
            alternative_objectives_probability,
            next_episode: AtomicUsize::new(1),
        }
    }

    #[cfg(test)]
    fn new_test(
        inner_search: TargetSearchOperator,
        recovery_operator: Arc<dyn Recreate>,
        max_repeat_count: usize,
        alternative_objectives_probability: (Float, Float),
    ) -> Self {
        Self::create(
            InfeasibleEducation::Mock(inner_search),
            recovery_operator,
            max_repeat_count,
            alternative_objectives_probability,
        )
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

impl HeuristicEscapeOperator for InfeasibleSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn escape(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        let refinement_ctx = heuristic_ctx;
        // Revisit a structurally distant relaxed resident when possible. Otherwise, seed the hidden archive from
        // the regular parent selected by Rosomaxa.
        let relaxed_parent =
            refinement_ctx.selected_relaxed(solution).filter(|parent| is_relaxed_parent_eligible(parent));
        let repeat_count = refinement_ctx.environment.random.uniform_int(1, self.max_repeat_count as i32) as usize;

        // First try the same bounded education under strict constraints. Crossing the feasibility boundary is useful
        // only when this strict pass cannot make immediate progress from the selected regular parent.
        if relaxed_parent.is_none() {
            let mut candidate = self.inner_search.search(refinement_ctx, solution);
            finalize_insertion_ctx(&mut candidate);

            if is_improvement(refinement_ctx, &candidate, solution) {
                return vec![candidate];
            }
        }

        let Some(mut new_insertion_ctx) =
            create_relaxed_insertion_ctx(relaxed_parent.unwrap_or(solution), self.alternative_objectives_probability)
        else {
            return vec![solution.deep_copy()];
        };
        let previous_state = relaxed_parent.and_then(|parent| parent.solution.state.get_relaxed_search()).copied();
        let mut search_state = previous_state.unwrap_or_else(|| {
            let needs_recovery = relaxed_parent.is_some();
            RelaxedSearchState::new(self.next_episode.fetch_add(1, AtomicOrdering::Relaxed), needs_recovery)
        });
        new_insertion_ctx.solution.state.set_relaxed_search(search_state);
        let mut new_refinement_ctx = create_relaxed_refinement_ctx(&new_insertion_ctx, self.max_repeat_count + 1);

        // Keep a retained lineage monotone: if none of its descendants is better under the tighter
        // violation band, the relaxed parent remains available instead of being forcibly replaced.
        if relaxed_parent.is_some() {
            new_refinement_ctx.add_solution(new_insertion_ctx.deep_copy());
        }

        let repeat_count = if relaxed_parent.is_some() { repeat_count } else { repeat_count.saturating_sub(1).max(1) };
        let objective = refinement_ctx.objective();
        let mut feasible_checkpoint = None;

        // Keep educating the endpoint reached by the previous repeat. The temporary population records
        // outcomes for selection, but must not branch this trajectory from an older checkpoint.
        let mut current = new_insertion_ctx;
        for _ in 0..repeat_count {
            let parent_state =
                *current.solution.state.get_relaxed_search().expect("relaxed search parent has checkpoint state");
            let education = self.inner_search.search_relaxed(&new_refinement_ctx, &current, objective);
            if let Some(candidate) = education.feasible_intermediate
                && feasible_checkpoint
                    .as_ref()
                    .is_none_or(|best| objective.total_order(&candidate, best) == Ordering::Less)
            {
                feasible_checkpoint = Some(candidate);
            }
            let mut new_insertion_ctx = education.endpoint.unwrap_or_else(|| current.deep_copy());
            finalize_insertion_ctx(&mut new_insertion_ctx);
            let parent_order = new_refinement_ctx.objective().total_order(&new_insertion_ctx, &current);

            // Exact violation degrees are derived only after every route and solution state has been refreshed.
            if new_insertion_ctx.problem.goal.is_relaxed_admissible(&new_insertion_ctx) {
                match parent_order {
                    Ordering::Equal => {
                        new_insertion_ctx.solution.state.set_relaxed_search(parent_state);
                        new_refinement_ctx.add_solution(new_insertion_ctx);
                    }
                    Ordering::Less | Ordering::Greater => {
                        search_state.advance();
                        new_insertion_ctx.solution.state.set_relaxed_search(search_state);
                        current = new_insertion_ctx.deep_copy();
                        new_refinement_ctx.add_solution(new_insertion_ctx);
                    }
                }
            }
        }

        let improved_feasible = new_refinement_ctx
            .ranked()
            .filter(|solution| get_violation(solution) == Some(0.))
            .min_by(|left, right| objective.total_order(left, right))
            .filter(|candidate| is_improvement(refinement_ctx, candidate, solution))
            .map(|candidate| candidate.deep_copy());

        let mut relaxed = new_refinement_ctx
            .ranked()
            .filter(|solution| get_violation(solution).is_some())
            .filter(|solution| {
                solution
                    .solution
                    .state
                    .get_relaxed_search()
                    .is_some_and(|state| state.progress.episode() == search_state.progress.episode())
            })
            // A zero-violation point is a useful continuation only after the episode has actually moved. Otherwise,
            // every unsuccessful fresh seed would add an unchanged copy to the hidden archive.
            .filter(|solution| {
                relaxed_parent.is_some()
                    || solution.solution.state.get_relaxed_search().is_some_and(|state| state.progress.step() > 0)
            })
            .max_by_key(|solution| solution.solution.state.get_relaxed_search().map(|state| state.progress))
            .map(|solution| solution.deep_copy());
        let mut regular = improved_feasible.map(|candidate| restore_feasible_individual(refinement_ctx, candidate));

        if let Some(relaxed) = relaxed.as_mut() {
            search_state = relaxed.solution.state.get_relaxed_search().copied().unwrap_or(search_state);

            // Reconstruct a checkpoint once, then wait until education changes it before paying that cost again.
            if regular.is_none()
                && relaxed_parent.is_some()
                && get_violation(relaxed).is_some_and(|violation| violation > 0.)
                && search_state.needs_recovery()
            {
                let recovered = self.recover_individual(refinement_ctx, relaxed.deep_copy());
                let parent_order = objective.total_order(&recovered, solution);
                let best_order = refinement_ctx.ranked().next().map(|best| objective.total_order(&recovered, best));
                if is_repair_improvement(refinement_ctx.selection_phase(), parent_order, best_order) {
                    regular = Some(recovered);
                }
                search_state.mark_recovered();
            }

            search_state.complete_visit();
            relaxed.solution.state.set_relaxed_search(search_state);
        }

        // Merge the retained intermediate only after the normal reconstruction attempt. Otherwise, finding an
        // intermediate would suppress recovery of the active endpoint, which can produce a better regular child.
        let feasible_checkpoint = feasible_checkpoint
            .filter(|candidate| is_improvement(refinement_ctx, candidate, solution))
            .map(|candidate| restore_feasible_individual(refinement_ctx, candidate));
        if let Some(candidate) = feasible_checkpoint
            && regular.as_ref().is_none_or(|best| objective.total_order(&candidate, best) == Ordering::Less)
        {
            regular = Some(candidate);
        }

        let mut offspring = Vec::with_capacity(2);
        offspring.extend(regular);
        offspring.extend(relaxed);
        if offspring.is_empty() {
            offspring.push(solution.deep_copy());
        }

        offspring
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

fn get_violation(insertion_ctx: &InsertionContext) -> Option<Float> {
    insertion_ctx.solution.state.get_relaxed_violation().copied()
}

fn get_infeasibility_tolerance(violation: Option<Float>) -> Float {
    violation
        .filter(|violation| *violation > 0.)
        .map_or(INITIAL_INFEASIBILITY_TOLERANCE, |violation| violation * INFEASIBILITY_TOLERANCE_CONTRACTION)
}

fn restore_feasible_individual(
    refinement_ctx: &RefinementContext,
    mut insertion_ctx: InsertionContext,
) -> InsertionContext {
    insertion_ctx.problem = refinement_ctx.problem.clone();
    insertion_ctx.solution.state.remove_value::<RelaxedSearchSolutionStateKey>();
    finalize_insertion_ctx(&mut insertion_ctx);

    insertion_ctx
}

fn create_relaxed_refinement_ctx(new_insertion_ctx: &InsertionContext, population_size: usize) -> RefinementContext {
    let problem = new_insertion_ctx.problem.clone();
    let environment = new_insertion_ctx.environment.clone();
    let population = Box::new(ElitismPopulation::new_with_dedup(
        problem.goal.clone(),
        environment.random.clone(),
        population_size,
        population_size,
        // Each accepted iteration has its own slot. Ordinary fitness alone cannot distinguish a feasible sibling
        // from the relaxed checkpoint whose lifecycle metadata still has to be returned to the main population.
        Box::new(|_, _, _| false),
    ));

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
    let violation = get_violation(insertion_ctx);
    let infeasibility_tolerance = get_infeasibility_tolerance(violation);
    let max_infeasibility = violation.unwrap_or_default().max(infeasibility_tolerance);
    let variant = Arc::new(alternative.relaxed(infeasibility_tolerance, max_infeasibility)?);

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
