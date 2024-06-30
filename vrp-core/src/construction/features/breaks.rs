//! A vehicle break features.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/breaks_test.rs"]
mod breaks_test;

use super::*;
use crate::construction::enablers::*;
use crate::models::solution::Route;
use std::collections::HashSet;
use std::iter::once;

/// Specifies break policy.
#[derive(Clone)]
pub enum BreakPolicy {
    /// Allows to skip break if actual tour schedule doesn't intersect with vehicle time window.
    SkipIfNoIntersection,

    /// Allows to skip break if vehicle arrives before break's time window end.
    SkipIfArrivalBeforeEnd,
}

/// Provides a way to build a feature to schedule an optional break. Here, optional means that break
/// sometimes can be skipped due to constraint violations or suboptimal search path in solution space.
pub struct BreakFeatureBuilder {
    name: String,
    violation_code: Option<ViolationCode>,
    belongs_to_route_fn: Option<BelongsToRouteFn>,
    is_break_single_fn: Option<BreakSingleFn>,
    policy_fn: Option<BreakPolicyFn>,
}

impl BreakFeatureBuilder {
    /// Creates a new instance of `BreakFeatureBuilder`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            violation_code: None,
            belongs_to_route_fn: None,
            is_break_single_fn: None,
            policy_fn: None,
        }
    }

    /// Sets constraint violation code which is used to report back the reason of job's unassignment.
    /// If not set, default violation code is used.
    pub fn set_violation_code(mut self, violation_code: ViolationCode) -> Self {
        self.violation_code = Some(violation_code);
        self
    }

    /// Sets a function which specifies whether a given single job can be considered as a break job.
    pub fn set_is_break_single<F>(mut self, func: F) -> Self
    where
        F: Fn(&Single) -> bool + Send + Sync + 'static,
    {
        self.is_break_single_fn = Some(Arc::new(func));
        self
    }

    /// Sets a break policy. If not set, then [BreakPolicy::SkipIfNoIntersection] is used.
    pub fn set_policy<F>(mut self, func: F) -> Self
    where
        F: Fn(&Single) -> BreakPolicy + Send + Sync + 'static,
    {
        self.policy_fn = Some(Arc::new(func));
        self
    }

    /// Sets a function which specifies whether a given route can serve a given break. This function
    /// should return false, if the job is not break. If not set, any break job can be assigned to any route.
    pub fn set_belongs_to_route<F>(mut self, func: F) -> Self
    where
        F: Fn(&Route, &Job) -> bool + Send + Sync + 'static,
    {
        self.belongs_to_route_fn = Some(Arc::new(func));
        self
    }

    /// Builds a optional break feature.
    pub fn build(mut self) -> GenericResult<Feature> {
        let is_break_single_fn =
            self.is_break_single_fn.take().ok_or_else(|| GenericError::from("is_break_single must be set"))?;

        let code = self.violation_code.take().unwrap_or_default();
        let policy_fn = self.policy_fn.take().unwrap_or_else(|| Arc::new(|_| BreakPolicy::SkipIfNoIntersection));
        let belongs_to_route_fn = self.belongs_to_route_fn.take().unwrap_or_else(|| {
            Arc::new({
                let is_break_single_fn = is_break_single_fn.clone();
                move |_, job| job.as_single().map_or(false, |single| is_break_single_fn(single))
            })
        });

        let break_fns = BreakFns { is_break_single_fn, belongs_to_route_fn, policy_fn };

        let context_transition = ConcreteJobContextTransition {
            remove_required: {
                let break_fns = break_fns.clone();
                move |solution_ctx, route_index, job| {
                    !is_required_job(&break_fns, solution_ctx.routes.as_slice(), route_index, job, true)
                }
            },
            promote_required: {
                let break_fns = break_fns.clone();
                move |solution_ctx, route_index, job| {
                    is_required_job(&break_fns, solution_ctx.routes.as_slice(), route_index, job, false)
                }
            },
            remove_locked: |_, _, _| false,
            promote_locked: |_, _, _| false,
        };

        FeatureBuilder::default()
            .with_name(self.name.as_str())
            .with_constraint(OptionalBreakConstraint { break_fns: break_fns.clone(), code })
            .with_objective(OptionalBreakObjective { break_fns: break_fns.clone() })
            .with_state(OptionalBreakState { context_transition, break_fns })
            .build()
    }
}

type BreakSingleFn = Arc<dyn Fn(&Single) -> bool + Send + Sync>;
type BelongsToRouteFn = Arc<dyn Fn(&Route, &Job) -> bool + Send + Sync>;
type BreakPolicyFn = Arc<dyn Fn(&Single) -> BreakPolicy + Send + Sync>;

#[derive(Clone)]
struct BreakFns {
    is_break_single_fn: BreakSingleFn,
    belongs_to_route_fn: BelongsToRouteFn,
    policy_fn: BreakPolicyFn,
}

struct OptionalBreakConstraint {
    break_fns: BreakFns,
    code: ViolationCode,
}

impl OptionalBreakConstraint {
    fn evaluate_route(&self, route_ctx: &RouteContext, job: &Job) -> Option<ConstraintViolation> {
        let single = job.as_single()?;

        // reject break for another vehicle
        if (self.break_fns.is_break_single_fn)(single) && !(self.break_fns.belongs_to_route_fn)(route_ctx.route(), job)
        {
            ConstraintViolation::fail(self.code)
        } else {
            None
        }
    }

    fn evaluate_activity(&self, activity_ctx: &ActivityContext) -> Option<ConstraintViolation> {
        activity_ctx
            .target
            .job
            .as_ref()
            // reject inserting break at the very beginning
            .filter(|single| (self.break_fns.is_break_single_fn)(single) && activity_ctx.prev.job.is_none())
            .and_then(|_| ConstraintViolation::skip(self.code))
    }
}

impl FeatureConstraint for OptionalBreakConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.evaluate_route(route_ctx, job),
            MoveContext::Activity { activity_ctx, .. } => self.evaluate_activity(activity_ctx),
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        let any_is_break = once(&source)
            .chain(once(&candidate))
            .filter_map(|job| job.as_single())
            .any(|single| (self.break_fns.is_break_single_fn)(single));

        if any_is_break {
            Err(self.code)
        } else {
            Ok(source)
        }
    }
}

struct OptionalBreakObjective {
    break_fns: BreakFns,
}

impl FeatureObjective for OptionalBreakObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| route_ctx.route().tour.jobs())
            .filter_map(|job| job.as_single())
            .filter(|single| (self.break_fns.is_break_single_fn)(single))
            .count() as f64
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { job, .. } => {
                if job.as_single().map_or(false, |single| (self.break_fns.is_break_single_fn)(single)) {
                    1.
                } else {
                    Cost::default()
                }
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct OptionalBreakState<JT: JobContextTransition + Send + Sync> {
    context_transition: JT,
    break_fns: BreakFns,
}

impl<JT: JobContextTransition + Send + Sync> FeatureState for OptionalBreakState<JT> {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        process_conditional_jobs(solution_ctx, Some(route_index), &self.context_transition);
    }

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        process_conditional_jobs(solution_ctx, None, &self.context_transition);
        self.remove_invalid_breaks(solution_ctx);
    }
}

impl<JT: JobContextTransition + Send + Sync> OptionalBreakState<JT> {
    /// Removes breaks which conditions are violated after ruin:
    /// * break without location served separately when original job is removed, but break is kept.
    /// * break is defined by interval, but its time is violated. This might happen due to departure time rescheduling.
    fn remove_invalid_breaks(&self, solution_ctx: &mut SolutionContext) {
        let breaks_to_remove = solution_ctx
            .routes
            .iter()
            .flat_map(|route_ctx| {
                route_ctx
                    .route()
                    .tour
                    .all_activities()
                    .fold((0, HashSet::new()), |(prev, mut breaks), activity| {
                        let current = activity.place.location;

                        let Some(break_single) = activity
                            .job
                            .as_ref()
                            .filter(|single| (self.break_fns.is_break_single_fn)(single))
                            .filter(|&single| !solution_ctx.locked.contains(&Job::Single(single.clone())))
                        else {
                            return (current, breaks);
                        };

                        // NOTE break should have location defined for all places or for none of them
                        let location_count = break_single.places.iter().filter(|p| p.location.is_some()).count();
                        assert!(
                            location_count == 0 || location_count == break_single.places.len(),
                            "break with multiple places is not supported"
                        );

                        let is_orphan =
                            prev != current && break_single.places.first().and_then(|p| p.location).is_none();
                        let is_not_on_time = !is_on_proper_time(route_ctx, break_single, &activity.schedule)
                            || !can_be_scheduled(route_ctx, break_single, &self.break_fns.policy_fn);
                        let is_ovrp_last =
                            route_ctx.route().tour.end().map_or(false, |end| std::ptr::eq(activity, end));

                        if is_orphan || is_not_on_time || is_ovrp_last {
                            breaks.insert(Job::Single(break_single.clone()));
                        }

                        (current, breaks)
                    })
                    .1
                    .into_iter()
            })
            .collect::<Vec<_>>();

        breaks_to_remove.iter().for_each(|break_job| {
            solution_ctx.routes.iter_mut().filter(|route_ctx| route_ctx.route().tour.contains(break_job)).for_each(
                |route_ctx| {
                    assert!(route_ctx.route_mut().tour.remove(break_job), "cannot remove break from the tour");
                },
            )
        });

        solution_ctx.unassigned.extend(breaks_to_remove.into_iter().map(|b| (b, UnassignmentInfo::Unknown)));

        // NOTE remove stale breaks from the violation list
        solution_ctx.unassigned.retain(|job, _| {
            let routes = solution_ctx.routes.as_slice();
            if !is_required_job(&self.break_fns, routes, None, job, true) {
                solution_ctx.ignored.push(job.clone());
                false
            } else {
                true
            }
        });
    }
}

fn is_required_job(
    break_fns: &BreakFns,
    routes: &[RouteContext],
    route_index: Option<usize>,
    job: &Job,
    default: bool,
) -> bool {
    job.as_single().map_or(default, |single| is_required_single(break_fns, routes, route_index, single, default))
}

/// Mark single job as ignored only if it has a break type and vehicle id is not present in routes
fn is_required_single(
    break_fns: &BreakFns,
    routes: &[RouteContext],
    route_index: Option<usize>,
    single: &Arc<Single>,
    default: bool,
) -> bool {
    if !(break_fns.is_break_single_fn)(single) {
        return default;
    }

    if let Some(route_index) = route_index {
        can_be_scheduled(&routes[route_index], single, &break_fns.policy_fn)
    } else {
        routes.iter().any(|route_ctx| {
            (break_fns.belongs_to_route_fn)(route_ctx.route(), &Job::Single(single.clone()))
                && can_be_scheduled(route_ctx, single, &break_fns.policy_fn)
        })
    }
}

/// Checks whether break can be scheduled in route.
fn can_be_scheduled(route_ctx: &RouteContext, break_single: &Single, policy_fn: &BreakPolicyFn) -> bool {
    let departure = route_ctx.route().tour.start().unwrap().schedule.departure;
    let arrival = route_ctx.route().tour.end().map_or(0., |end| end.schedule.arrival);
    let tour_tw = TimeWindow::new(departure, arrival);

    let policy = policy_fn(break_single);

    get_break_time_windows(break_single, departure).any(|break_tw| match policy {
        BreakPolicy::SkipIfNoIntersection => break_tw.intersects(&tour_tw),
        BreakPolicy::SkipIfArrivalBeforeEnd => tour_tw.end > break_tw.end,
    })
}

fn get_break_time_windows(break_single: &'_ Single, departure: f64) -> impl Iterator<Item = TimeWindow> + '_ {
    break_single
        .places
        .first()
        .expect("missing time window in a break job")
        .times
        .iter()
        .map(move |span| span.to_time_window(departure))
}

/// Checks whether break is scheduled on time as its time can be invalid due to departure time optimizations.
fn is_on_proper_time(route_ctx: &RouteContext, break_job: &Single, actual_schedule: &Schedule) -> bool {
    let departure = route_ctx.route().tour.start().unwrap().schedule.departure;
    let actual_tw = TimeWindow::new(actual_schedule.arrival, actual_schedule.departure);

    get_break_time_windows(break_job, departure).any(|tw| tw.intersects(&actual_tw))
}
