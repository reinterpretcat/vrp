#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/conditional_test.rs"]
mod conditional_test;

use crate::construction::constraints::{ConstraintModule, ConstraintVariant};
use crate::construction::heuristics::{RouteContext, SolutionContext};
use crate::models::problem::Job;
use hashbrown::HashSet;
use std::slice::Iter;

/// Defines how jobs are moved in solution context. Index of original affected route context is passed.
pub trait JobContextTransition {
    /// Returns true if job is moved from required to ignored.
    fn remove_from_required(&self, solution_ctx: &SolutionContext, route_index: Option<usize>, job: &Job) -> bool;

    /// Returns true if job is moved from ignored to required.
    fn promote_to_required(&self, solution_ctx: &SolutionContext, route_index: Option<usize>, job: &Job) -> bool;

    /// Returns true if job is removed from locked.
    fn remove_from_locked(&self, solution_ctx: &SolutionContext, route_index: Option<usize>, job: &Job) -> bool;

    /// Returns true if job is moved to locked.
    fn promote_to_locked(&self, solution_ctx: &SolutionContext, route_index: Option<usize>, job: &Job) -> bool;
}

/// A concrete implementation of `JobContextTransition` which allows to use lambdas.
pub struct ConcreteJobContextTransition<FRemoveRequired, FPromoteRequired, FRemoveLocked, FPromoteLocked>
where
    FRemoveRequired: Fn(&SolutionContext, Option<usize>, &Job) -> bool,
    FPromoteRequired: Fn(&SolutionContext, Option<usize>, &Job) -> bool,
    FRemoveLocked: Fn(&SolutionContext, Option<usize>, &Job) -> bool,
    FPromoteLocked: Fn(&SolutionContext, Option<usize>, &Job) -> bool,
{
    /// A function which removes job from required list.
    pub remove_required: FRemoveRequired,
    /// A function which promotes job to required jobs.
    pub promote_required: FPromoteRequired,
    /// A function which removes job from locked list.
    pub remove_locked: FRemoveLocked,
    /// A function which promotes job to locked jobs.
    pub promote_locked: FPromoteLocked,
}

impl<FRemoveRequired, FPromoteRequired, FRemoveLocked, FPromoteLocked> JobContextTransition
    for ConcreteJobContextTransition<FRemoveRequired, FPromoteRequired, FRemoveLocked, FPromoteLocked>
where
    FRemoveRequired: Fn(&SolutionContext, Option<usize>, &Job) -> bool,
    FPromoteRequired: Fn(&SolutionContext, Option<usize>, &Job) -> bool,
    FRemoveLocked: Fn(&SolutionContext, Option<usize>, &Job) -> bool,
    FPromoteLocked: Fn(&SolutionContext, Option<usize>, &Job) -> bool,
{
    fn remove_from_required(&self, solution_ctx: &SolutionContext, route_index: Option<usize>, job: &Job) -> bool {
        (self.remove_required)(solution_ctx, route_index, job)
    }

    fn promote_to_required(&self, solution_ctx: &SolutionContext, route_index: Option<usize>, job: &Job) -> bool {
        (self.promote_required)(solution_ctx, route_index, job)
    }

    fn remove_from_locked(&self, solution_ctx: &SolutionContext, route_index: Option<usize>, job: &Job) -> bool {
        (self.remove_locked)(solution_ctx, route_index, job)
    }

    fn promote_to_locked(&self, solution_ctx: &SolutionContext, route_index: Option<usize>, job: &Job) -> bool {
        (self.promote_locked)(solution_ctx, route_index, job)
    }
}

/// A module which allows to promote jobs between required and ignored collection using some condition.
/// Useful to model some optional/conditional activities, e.g. breaks, refueling, etc.
pub struct ConditionalJobModule {
    context_transition: Box<dyn JobContextTransition + Send + Sync>,
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
}

impl ConditionalJobModule {
    /// Creates a new instance of `ConditionalJobModule`.
    pub fn new(context_transition: Box<dyn JobContextTransition + Send + Sync>) -> Self {
        Self { context_transition, state_keys: vec![], constraints: vec![] }
    }
}

impl ConstraintModule for ConditionalJobModule {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _job: &Job) {
        // TODO avoid calling this on each insertion as it is expensive.
        analyze_and_perform_transitions(solution_ctx, Some(route_index), self.context_transition.as_ref());
    }

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        analyze_and_perform_transitions(solution_ctx, None, self.context_transition.as_ref());
    }

    fn merge(&self, source: Job, _candidate: Job) -> Result<Job, i32> {
        Ok(source)
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

fn analyze_and_perform_transitions(
    solution_ctx: &mut SolutionContext,
    route_index: Option<usize>,
    context_transition: &(dyn JobContextTransition + Send + Sync),
) {
    // analyzed required/ignored
    let ignored: HashSet<Job> = solution_ctx
        .required
        .iter()
        .filter(|job| context_transition.remove_from_required(solution_ctx, route_index, job))
        .cloned()
        .collect();
    solution_ctx.required.retain(|job| !ignored.contains(job));
    solution_ctx.unassigned.retain(|job, _| !ignored.contains(job));

    // identify required inside ignored
    let required: HashSet<Job> = solution_ctx
        .ignored
        .iter()
        .filter(|job| context_transition.promote_to_required(solution_ctx, route_index, job))
        .cloned()
        .collect();
    solution_ctx.ignored.retain(|job| !required.contains(job));

    solution_ctx.required.extend(required);
    solution_ctx.ignored.extend(ignored);

    // analyze locked
    let not_locked: HashSet<Job> = solution_ctx
        .locked
        .iter()
        .filter(|job| context_transition.remove_from_locked(solution_ctx, route_index, job))
        .cloned()
        .collect();
    solution_ctx.locked.retain(|job| !not_locked.contains(job));

    let locked: HashSet<Job> = solution_ctx
        .required
        .iter()
        .chain(solution_ctx.ignored.iter())
        .filter(|job| context_transition.promote_to_locked(solution_ctx, route_index, job))
        .cloned()
        .collect();

    solution_ctx.locked.extend(locked);
}
