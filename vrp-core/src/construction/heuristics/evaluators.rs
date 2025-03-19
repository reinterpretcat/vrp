#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/evaluators_test.rs"]
mod evaluators_test;

use rosomaxa::prelude::UnwrapValue;
use std::ops::ControlFlow;
use std::sync::Arc;

use crate::construction::heuristics::*;
use crate::models::common::Timestamp;
use crate::models::problem::{Job, Multi, Single};
use crate::models::solution::{Activity, Leg, Place};
use crate::models::{ConstraintViolation, GoalContext, ViolationCode};
use crate::utils::Either;

/// Specifies an evaluation context data.
pub struct EvaluationContext<'a> {
    /// An actual optimization goal context.
    pub goal: &'a GoalContext,
    /// A job which is about to be inserted.
    pub job: &'a Job,
    /// A leg selection mode.
    pub leg_selection: &'a LegSelection,
    /// A result selector.
    pub result_selector: &'a (dyn ResultSelector),
}

/// Specifies allowed insertion position in route for the job.
#[derive(Copy, Clone)]
pub enum InsertionPosition {
    /// Job can be inserted anywhere in the route.
    Any,
    /// Job can be inserted only at the leg with the concrete index.
    Concrete(usize),
    /// Job can be inserted only to the end of the route.
    Last,
}

/// Evaluates possibility to preform insertion from given insertion context in given route
/// at given position constraint.
pub fn eval_job_insertion_in_route(
    insertion_ctx: &InsertionContext,
    eval_ctx: &EvaluationContext,
    route_ctx: &RouteContext,
    position: InsertionPosition,
    alternative: InsertionResult,
) -> InsertionResult {
    // NOTE do not evaluate unassigned job in unmodified route if it has a concrete code
    match (route_ctx.is_stale(), insertion_ctx.solution.unassigned.get(eval_ctx.job)) {
        (false, Some(UnassignmentInfo::Simple(_))) | (false, Some(UnassignmentInfo::Detailed(_))) => {
            return alternative;
        }
        _ => {}
    }

    let goal = &insertion_ctx.problem.goal;

    // evaluate constraint violation on route level
    let penalty_cost: Option<InsertionCost> =
        match goal.evaluate(&MoveContext::route(&insertion_ctx.solution, route_ctx, eval_ctx.job)) {
            Some(ConstraintViolation { degree: Some(_), .. }) => {
                todo!("handle degree constraint violation")
            }
            Some(violation) => {
                return eval_ctx.result_selector.select_insertion(
                    insertion_ctx,
                    alternative,
                    InsertionResult::make_failure_with_code(violation.code, true, Some(eval_ctx.job.clone())),
                );
            }
            None => None,
        };

    // get route costs
    let route_cost = goal.estimate(&MoveContext::route(&insertion_ctx.solution, route_ctx, eval_ctx.job));
    let route_cost = goal.reconcile(route_cost, penalty_cost.as_ref());

    // analyze alternative and return it if it looks better based on routing cost comparison
    let (route_costs, best_known_cost) = match alternative.as_success() {
        Some(success) => {
            let alt_costs = goal.reconcile(success.cost.clone(), success.penalty.as_ref());
            match eval_ctx.result_selector.select_cost(&alt_costs, &route_cost) {
                Either::Left(_) => return alternative,
                Either::Right(_) => (route_cost, Some(alt_costs)),
            }
        }
        _ => (route_cost, None),
    };

    let solution_ctx = &insertion_ctx.solution;

    let move_cost = MoveCost { cost: route_costs, penalty_cost };

    eval_ctx.result_selector.select_insertion(
        insertion_ctx,
        alternative,
        eval_job_constraint_in_route(eval_ctx, solution_ctx, route_ctx, position, move_cost, best_known_cost),
    )
}

/// Evaluates possibility to preform insertion in route context only.
/// NOTE: doesn't evaluate constraints on route level.
fn eval_job_constraint_in_route(
    eval_ctx: &EvaluationContext,
    solution_ctx: &SolutionContext,
    route_ctx: &RouteContext,
    position: InsertionPosition,
    move_cost: MoveCost,
    best_known_cost: Option<InsertionCost>,
) -> InsertionResult {
    match eval_ctx.job {
        Job::Single(single) => {
            eval_single(eval_ctx, solution_ctx, route_ctx, single, position, move_cost, best_known_cost)
        }
        Job::Multi(multi) => eval_multi(eval_ctx, solution_ctx, route_ctx, multi, position, move_cost, best_known_cost),
    }
}

pub(crate) fn eval_single_constraint_in_route(
    insertion_ctx: &InsertionContext,
    eval_ctx: &EvaluationContext,
    route_ctx: &RouteContext,
    single: &Arc<Single>,
    position: InsertionPosition,
    move_cost: MoveCost,
    best_known_cost: Option<InsertionCost>,
) -> InsertionResult {
    let solution_ctx = &insertion_ctx.solution;

    match eval_ctx.goal.evaluate(&MoveContext::route(&insertion_ctx.solution, route_ctx, eval_ctx.job)) {
        Some(violation) => InsertionResult::Failure(InsertionFailure {
            constraint: violation.code,
            stopped: true,
            job: Some(eval_ctx.job.clone()),
        }),
        _ => eval_single(eval_ctx, solution_ctx, route_ctx, single, position, move_cost, best_known_cost),
    }
}

fn eval_single(
    eval_ctx: &EvaluationContext,
    solution_ctx: &SolutionContext,
    route_ctx: &RouteContext,
    single: &Arc<Single>,
    position: InsertionPosition,
    move_cost: MoveCost,
    best_known_cost: Option<InsertionCost>,
) -> InsertionResult {
    let insertion_idx = get_insertion_index(route_ctx, position);
    let mut activity = Activity::new_with_job(single.clone());

    let result = analyze_insertion_in_route(
        eval_ctx,
        solution_ctx,
        route_ctx,
        insertion_idx,
        single,
        &mut activity,
        move_cost,
        SingleContext::new(best_known_cost, 0),
    );

    let job = eval_ctx.job.clone();
    if let Some(place) = result.place {
        activity.place = place;
        let activities = vec![(activity, result.index)];
        let cost = result.cost.unwrap_or_default();
        let penalty = result.penalty_cost;

        InsertionResult::make_success(cost, job, activities, route_ctx, penalty)
    } else {
        let (code, stopped) = result.violation.map_or((ViolationCode::unknown(), false), |v| (v.code, v.stopped));
        InsertionResult::make_failure_with_code(code, stopped, Some(job))
    }
}

fn eval_multi(
    eval_ctx: &EvaluationContext,
    solution_ctx: &SolutionContext,
    route_ctx: &RouteContext,
    multi: &Arc<Multi>,
    position: InsertionPosition,
    move_cost: MoveCost,
    best_known_cost: Option<InsertionCost>,
) -> InsertionResult {
    let insertion_idx = get_insertion_index(route_ctx, position).unwrap_or(0);
    // 1. analyze permutations
    let result = multi
        .permutations()
        .into_iter()
        .try_fold(MultiContext::new(best_known_cost, insertion_idx), |acc_res, services| {
            let mut shadow = ShadowContext::new(eval_ctx.goal, route_ctx);
            let perm_res = (0..)
                .try_fold(MultiContext::new(None, insertion_idx), |out, _| {
                    if out.is_failure(route_ctx.route().tour.job_activity_count()) {
                        return ControlFlow::Break(out);
                    }
                    shadow.restore(route_ctx);

                    // 2. analyze inner jobs
                    let sq_res = services
                        .iter()
                        .try_fold(out.next(), |in1, service| {
                            if in1.violation.is_some() {
                                return ControlFlow::Break(in1);
                            }
                            let mut activity = Activity::new_with_job(service.clone());
                            // 3. analyze legs
                            let srv_res = analyze_insertion_in_route(
                                eval_ctx,
                                solution_ctx,
                                shadow.route_ctx(),
                                None,
                                service,
                                &mut activity,
                                Default::default(),
                                SingleContext::new(None, in1.next_index),
                            );

                            if let Some(place) = srv_res.place {
                                activity.place = place;
                                let activity = shadow.insert(activity, srv_res.index);
                                let activities = concat_activities(in1.activities, (activity, srv_res.index));

                                let cost = in1.cost.unwrap_or_else(|| move_cost.cost.clone())
                                    + srv_res.cost.unwrap_or_default();

                                let penalty_cost =
                                    [in1.penalty_cost.or_else(|| move_cost.penalty_cost.clone()), srv_res.penalty_cost]
                                        .into_iter()
                                        .flatten()
                                        .reduce(|acc, i| acc + i);

                                return MultiContext::success(cost, activities, penalty_cost);
                            }

                            MultiContext::fail(srv_res, in1)
                        })
                        .unwrap_value();

                    MultiContext::promote(sq_res, out)
                })
                .unwrap_value();

            MultiContext::promote(perm_res, acc_res)
        })
        .unwrap_value();

    let job = eval_ctx.job.clone();
    if result.is_success() {
        let activities = result.activities.unwrap_or_default();
        let cost = result.cost.unwrap_or_default();
        let penalty_cost = result.penalty_cost;

        InsertionResult::make_success(cost, job, activities, route_ctx, penalty_cost)
    } else {
        let (code, stopped) = result.violation.map_or((ViolationCode::unknown(), false), |v| (v.code, v.stopped));
        InsertionResult::make_failure_with_code(code, stopped, Some(job))
    }
}

#[allow(clippy::too_many_arguments)]
fn analyze_insertion_in_route(
    eval_ctx: &EvaluationContext,
    solution_ctx: &SolutionContext,
    route_ctx: &RouteContext,
    insertion_idx: Option<usize>,
    single: &Single,
    target: &mut Activity,
    move_cost: MoveCost,
    init: SingleContext,
) -> SingleContext {
    let mut analyze_leg_insertion = |leg: Leg<'_>, init| {
        analyze_insertion_in_route_leg(eval_ctx, solution_ctx, route_ctx, leg, single, target, move_cost.clone(), init)
    };

    match insertion_idx {
        Some(idx) => match route_ctx.route().tour.legs().nth(idx) {
            Some(leg) => analyze_leg_insertion(leg, init).unwrap_value(),
            _ => init,
        },
        None => eval_ctx.leg_selection.sample_best(
            route_ctx,
            eval_ctx.job,
            init.index,
            init,
            &mut |leg: Leg<'_>, init| analyze_leg_insertion(leg, init),
            {
                let max_value = InsertionCost::max_value();
                move |lhs: &SingleContext, rhs: &SingleContext| {
                    eval_ctx
                        .result_selector
                        .select_cost(lhs.cost.as_ref().unwrap_or(max_value), rhs.cost.as_ref().unwrap_or(max_value))
                        .is_left()
                }
            },
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn analyze_insertion_in_route_leg(
    eval_ctx: &EvaluationContext,
    solution_ctx: &SolutionContext,
    route_ctx: &RouteContext,
    leg: Leg,
    single: &Single,
    target: &mut Activity,
    move_cost: MoveCost,
    mut single_ctx: SingleContext,
) -> ControlFlow<SingleContext, SingleContext> {
    let (items, index) = leg;
    let (prev, next) = match items {
        [prev] => (prev, None),
        [prev, next] => (prev, Some(next)),
        _ => return ControlFlow::Break(single_ctx),
    };
    let start_time = route_ctx.route().tour.start().map_or(Timestamp::default(), |act| act.schedule.departure);

    // iterate over places and times to find the next best insertion point
    for (place_idx, place) in single.places.iter().enumerate() {
        target.place.idx = place_idx;
        target.place.location = place.location.unwrap_or(prev.place.location);
        target.place.duration = place.duration;

        // iterate over time windows of the place
        for time in place.times.iter() {
            target.place.time = time.to_time_window(start_time);

            let activity_ctx = ActivityContext { index, prev, target, next };
            let move_ctx = MoveContext::activity(solution_ctx, route_ctx, &activity_ctx);

            // determine evaluation costs if they are present
            let penalty_cost = match eval_ctx.goal.evaluate(&move_ctx) {
                Some(ConstraintViolation { degree: Some(_), .. }) => {
                    todo!("handle degree constraint violation")
                }
                Some(violation) if violation.stopped => {
                    single_ctx.violation = Some(violation);
                    return ControlFlow::Break(single_ctx);
                }
                Some(violation) => {
                    single_ctx.violation = Some(violation);
                    continue;
                }
                None => None,
            };

            // get final cost estimation
            let costs = eval_ctx.goal.estimate(&move_ctx) + &move_cost.cost;
            let costs = eval_ctx.goal.reconcile(costs, penalty_cost.as_ref());

            let other_costs = single_ctx.cost.as_ref().unwrap_or(InsertionCost::max_value());

            match eval_ctx.result_selector.select_cost(&costs, other_costs) {
                // found better insertion
                Either::Left(_) => {
                    single_ctx.violation = None;
                    single_ctx.index = index;
                    single_ctx.cost = Some(costs);
                    single_ctx.penalty_cost = penalty_cost;
                    single_ctx.place = Some(target.place.clone());
                }
                Either::Right(_) => continue,
            }
        }
    }

    ControlFlow::Continue(single_ctx)
}

fn get_insertion_index(route_ctx: &RouteContext, position: InsertionPosition) -> Option<usize> {
    match position {
        InsertionPosition::Any => None,
        InsertionPosition::Concrete(idx) => Some(idx),
        InsertionPosition::Last => Some(route_ctx.route().tour.legs().count().max(1) - 1),
    }
}

/// Keeps track of cost estimations of the move (insertion)
#[derive(Clone, Default)]
pub(crate) struct MoveCost {
    /// Estimation specifies delta cost change for the insertion.
    cost: InsertionCost,
    /// Penalty specifies (in)feasibility degree (if not [None]).
    penalty_cost: Option<InsertionCost>,
}

/// Stores information needed for single insertion.
#[derive(Clone, Debug, Default)]
struct SingleContext {
    /// Constraint violation.
    pub violation: Option<ConstraintViolation>,
    /// Insertion index.
    pub index: usize,
    /// Best cost.
    pub cost: Option<InsertionCost>,
    /// Penalty metric.
    pub penalty_cost: Option<InsertionCost>,
    /// Activity place.
    pub place: Option<Place>,
}

impl SingleContext {
    /// Creates a new empty context with given cost.
    fn new(cost: Option<InsertionCost>, index: usize) -> Self {
        Self { violation: None, index, cost, place: None, penalty_cost: None }
    }
}

/// Stores information needed for multi job insertion.
struct MultiContext {
    /// Constraint violation.
    pub violation: Option<ConstraintViolation>,
    /// Insertion index for first service.
    pub start_index: usize,
    /// Insertion index for next service.
    pub next_index: usize,
    /// Cost accumulator.
    pub cost: Option<InsertionCost>,
    /// Feasibility metric.
    pub penalty_cost: Option<InsertionCost>,
    /// Activities with their indices.
    pub activities: Option<Vec<(Activity, usize)>>,
}

impl MultiContext {
    /// Creates new empty insertion context.
    fn new(cost: Option<InsertionCost>, index: usize) -> Self {
        Self { violation: None, start_index: index, next_index: index, cost, penalty_cost: None, activities: None }
    }

    /// Promotes insertion context by best price.
    fn promote(left: Self, right: Self) -> ControlFlow<Self, Self> {
        let index = left.start_index.max(right.start_index) + 1;
        let best = match (&left.cost, &right.cost) {
            (Some(left_cost), Some(right_cost)) => {
                if left_cost < right_cost {
                    left
                } else {
                    right
                }
            }
            (Some(_), None) => left,
            (None, Some(_)) => right,
            // NOTE: no costs means failure, let's provide one which has violation field populated
            (None, None) => {
                if left.violation.is_some() {
                    left
                } else {
                    right
                }
            }
        };

        let result = Self {
            violation: best.violation,
            start_index: index,
            next_index: index,
            cost: best.cost,
            penalty_cost: best.penalty_cost,
            activities: best.activities,
        };

        if result.violation.as_ref().is_some_and(|v| v.stopped) {
            ControlFlow::Break(result)
        } else {
            ControlFlow::Continue(result)
        }
    }

    /// Creates failed insertion context within reason code.
    #[inline]
    fn fail(err_ctx: SingleContext, other_ctx: MultiContext) -> ControlFlow<Self, Self> {
        let violation = err_ctx
            .violation
            .map(|v| ConstraintViolation { stopped: v.stopped && other_ctx.activities.is_none(), ..v })
            .or_else(|| ConstraintViolation::skip(ViolationCode::unknown()));

        ControlFlow::Break(Self {
            violation,
            start_index: other_ctx.start_index,
            next_index: other_ctx.start_index,
            cost: None,
            penalty_cost: None,
            activities: None,
        })
    }

    /// Creates successful insertion context.
    #[inline]
    fn success(
        cost: InsertionCost,
        activities: Vec<(Activity, usize)>,
        penalty_cost: Option<InsertionCost>,
    ) -> ControlFlow<Self, Self> {
        // NOTE avoid stack unwinding
        let start_index = activities.first().map_or(0, |act| act.1);
        let next_index = activities.last().map_or(0, |act| act.1) + 1;

        ControlFlow::Continue(Self {
            violation: None,
            start_index,
            next_index,
            cost: Some(cost),
            penalty_cost,
            activities: Some(activities),
        })
    }

    /// Creates next insertion context from existing one.
    #[inline]
    fn next(&self) -> Self {
        Self {
            violation: None,
            start_index: self.start_index,
            next_index: self.start_index,
            cost: None,
            penalty_cost: None,
            activities: None,
        }
    }

    /// Checks whether insertion is found.
    #[inline]
    fn is_success(&self) -> bool {
        self.violation.is_none() && self.cost.is_some() && self.activities.is_some()
    }

    /// Checks whether insertion is failed.
    #[inline]
    fn is_failure(&self, index: usize) -> bool {
        self.violation.as_ref().is_some_and(|v| v.stopped) || (self.start_index > index)
    }
}

/// Provides the way to use copy on write strategy within route state context.
struct ShadowContext<'a> {
    goal: &'a GoalContext,
    // NOTE Cow might be a better fit, but it would require RouteContext to implement Clone trait.
    //      However we want to avoid this as it might lead to unnecessary clones and, as result,
    //      performance degradation.
    ctx: Either<&'a RouteContext, RouteContext>,
}

impl<'a> ShadowContext<'a> {
    fn new(goal: &'a GoalContext, ctx: &'a RouteContext) -> Self {
        Self { goal, ctx: Either::Left(ctx) }
    }

    fn route_ctx(&self) -> &'_ RouteContext {
        match &self.ctx {
            Either::Left(route_ctx) => route_ctx,
            Either::Right(route_ctx) => route_ctx,
        }
    }

    fn insert(&mut self, activity: Activity, index: usize) -> Activity {
        if let Either::Left(route_ctx) = &self.ctx {
            self.ctx = Either::Right(route_ctx.deep_copy());
        }

        if let Either::Right(ref mut route_ctx) = self.ctx {
            route_ctx.route_mut().tour.insert_at(activity.deep_copy(), index + 1);
            self.goal.accept_route_state(route_ctx);
        }

        activity
    }

    fn restore(&mut self, original: &'a RouteContext) {
        if let Either::Right(_) = &self.ctx {
            self.ctx = Either::Left(original)
        }
    }
}

fn concat_activities(
    activities: Option<Vec<(Activity, usize)>>,
    activity: (Activity, usize),
) -> Vec<(Activity, usize)> {
    let mut activities = activities.unwrap_or_default();
    activities.push((activity.0, activity.1));

    activities
}
