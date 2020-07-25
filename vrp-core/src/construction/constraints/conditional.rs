#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/conditional_test.rs"]
mod conditional_test;

use crate::construction::constraints::{ConstraintModule, ConstraintVariant};
use crate::construction::heuristics::{RouteContext, SolutionContext};
use crate::models::problem::Job;
use hashbrown::HashSet;
use std::slice::Iter;

/// Defines how jobs are moved in context.
pub trait JobContextTransition {
    /// Returns true if job is moved from required to ignored.
    fn remove_from_required(&self, ctx: &SolutionContext, job: &Job) -> bool;

    /// Returns true if job is moved from ignored to required.
    fn promote_to_required(&self, ctx: &SolutionContext, job: &Job) -> bool;

    /// Returns true if job is removed from locked.
    fn remove_from_locked(&self, ctx: &SolutionContext, job: &Job) -> bool;

    /// Returns true if job is moved to locked.
    fn promote_to_locked(&self, ctx: &SolutionContext, job: &Job) -> bool;
}

/// A concrete implementation of `JobContextTransition` which allows to use lambdas.
pub struct ConcreteJobContextTransition<FRemoveRequired, FPromoteRequired, FRemoveLocked, FPromoteLocked>
where
    FRemoveRequired: Fn(&SolutionContext, &Job) -> bool,
    FPromoteRequired: Fn(&SolutionContext, &Job) -> bool,
    FRemoveLocked: Fn(&SolutionContext, &Job) -> bool,
    FPromoteLocked: Fn(&SolutionContext, &Job) -> bool,
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
    FRemoveRequired: Fn(&SolutionContext, &Job) -> bool,
    FPromoteRequired: Fn(&SolutionContext, &Job) -> bool,
    FRemoveLocked: Fn(&SolutionContext, &Job) -> bool,
    FPromoteLocked: Fn(&SolutionContext, &Job) -> bool,
{
    fn remove_from_required(&self, ctx: &SolutionContext, job: &Job) -> bool {
        (self.remove_required)(ctx, job)
    }

    fn promote_to_required(&self, ctx: &SolutionContext, job: &Job) -> bool {
        (self.promote_required)(ctx, job)
    }

    fn remove_from_locked(&self, ctx: &SolutionContext, job: &Job) -> bool {
        (self.remove_locked)(ctx, job)
    }

    fn promote_to_locked(&self, ctx: &SolutionContext, job: &Job) -> bool {
        (self.promote_locked)(ctx, job)
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
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, _route_index: usize, _job: &Job) {
        // TODO avoid calling this on each insertion as it is expensive.
        self.accept_solution_state(solution_ctx);
    }

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        // analyzed required/ignored
        let ignored: HashSet<Job> =
            ctx.required.iter().filter(|job| self.context_transition.remove_from_required(ctx, job)).cloned().collect();
        ctx.required.retain(|job| !ignored.contains(job));

        // identify required inside ignored
        let required: HashSet<Job> =
            ctx.ignored.iter().filter(|job| self.context_transition.promote_to_required(ctx, job)).cloned().collect();
        ctx.ignored.retain(|job| !required.contains(job));

        ctx.required.extend(required);
        ctx.ignored.extend(ignored);

        // analyze locked
        let not_locked: HashSet<Job> =
            ctx.locked.iter().filter(|job| self.context_transition.remove_from_locked(ctx, job)).cloned().collect();
        ctx.locked.retain(|job| !not_locked.contains(job));

        let locked: HashSet<Job> = ctx
            .required
            .iter()
            .chain(ctx.ignored.iter())
            .filter(|job| self.context_transition.promote_to_locked(ctx, job))
            .cloned()
            .collect();

        ctx.locked.extend(locked);
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}
