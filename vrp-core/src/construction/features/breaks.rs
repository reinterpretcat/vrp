//! A vehicle break features.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/breaks_test.rs"]
mod breaks_test;

use super::*;
use crate::construction::enablers::*;
use hashbrown::HashSet;
use std::iter::once;

/// A helper type to work with a break job represented as a reference to `Single` or `Job` types.
#[derive(Clone, Copy)]
pub enum BreakCandidate<'a> {
    /// Single job type.
    Single(&'a Single),
    /// Common job type.
    Job(&'a Job),
}

impl BreakCandidate<'_> {
    /// Returns candidate as single job if possible.
    pub fn as_single(&self) -> Option<&Single> {
        match self {
            BreakCandidate::Job(Job::Single(single)) => Some(single.as_ref()),
            BreakCandidate::Single(single) => Some(single),
            _ => None,
        }
    }
}

/// Provides way to work with a break job.
pub trait BreakAspects: Clone + Send + Sync {
    /// Checks whether the candidate job is a break job.
    fn is_break_job(&self, candidate: BreakCandidate<'_>) -> bool;

    /// Checks whether the job is a break job and it can be assigned to the given route.
    fn belongs_to_route(&self, route_ctx: &RouteContext, candidate: BreakCandidate<'_>) -> bool;

    /// Gets break policy if it is defined.
    fn get_policy(&self, candidate: BreakCandidate<'_>) -> Option<BreakPolicy>;
}

/// Specifies break policy.
#[derive(Clone)]
pub enum BreakPolicy {
    /// Allows to skip break if actual tour schedule doesn't intersect with vehicle time window.
    SkipIfNoIntersection,

    /// Allows to skip break if vehicle arrives before break's time window end.
    SkipIfArrivalBeforeEnd,
}

/// Creates a feature to schedule an optional break. Here, optional means that break sometimes can
/// be skipped due to constraint violations or suboptimal search path in solution space.
pub fn create_optional_break_feature<T: BreakAspects + 'static>(
    name: &str,
    code: ViolationCode,
    aspects: T,
) -> Result<Feature, GenericError> {
    let context_transition = ConcreteJobContextTransition {
        remove_required: {
            let aspects = aspects.clone();
            move |solution_ctx, route_index, job| {
                !is_required_job(&aspects, solution_ctx.routes.as_slice(), route_index, job, true)
            }
        },
        promote_required: {
            let aspects = aspects.clone();
            move |solution_ctx, route_index, job| {
                is_required_job(&aspects, solution_ctx.routes.as_slice(), route_index, job, false)
            }
        },
        remove_locked: |_, _, _| false,
        promote_locked: |_, _, _| false,
    };

    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(OptionalBreakConstraint { code, aspects: aspects.clone() })
        .with_objective(OptionalBreakObjective { aspects: aspects.clone() })
        .with_state(OptionalBreakState { context_transition, aspects })
        .build()
}

struct OptionalBreakConstraint<BA: BreakAspects> {
    code: ViolationCode,
    aspects: BA,
}

impl<BA: BreakAspects> OptionalBreakConstraint<BA> {
    fn evaluate_route(&self, route_ctx: &RouteContext, job: &Job) -> Option<ConstraintViolation> {
        let candidate = BreakCandidate::Job(job);

        // reject break for another vehicle
        if self.aspects.is_break_job(candidate) && !self.aspects.belongs_to_route(route_ctx, candidate) {
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
            .filter(|single| {
                self.aspects.is_break_job(BreakCandidate::Single(single)) && activity_ctx.prev.job.is_none()
            })
            .and_then(|_| ConstraintViolation::skip(self.code))
    }
}

impl<BA: BreakAspects> FeatureConstraint for OptionalBreakConstraint<BA> {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.evaluate_route(route_ctx, job),
            MoveContext::Activity { activity_ctx, .. } => self.evaluate_activity(activity_ctx),
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        let any_is_break =
            once(&source).chain(once(&candidate)).any(|job| self.aspects.is_break_job(BreakCandidate::Job(job)));

        if any_is_break {
            Err(self.code)
        } else {
            Ok(source)
        }
    }
}

struct OptionalBreakObjective<BA: BreakAspects> {
    aspects: BA,
}

impl<BA: BreakAspects> FeatureObjective for OptionalBreakObjective<BA> {
    fn fitness(&self, solution: &InsertionContext) -> f64 {
        solution
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| route_ctx.route().tour.jobs())
            .filter(|job| self.aspects.is_break_job(BreakCandidate::Job(job)))
            .count() as f64
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { job, .. } => {
                if self.aspects.is_break_job(BreakCandidate::Job(job)) {
                    1.
                } else {
                    Cost::default()
                }
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct OptionalBreakState<BA: BreakAspects, JT: JobContextTransition + Send + Sync> {
    aspects: BA,
    context_transition: JT,
}

impl<BA: BreakAspects, JT: JobContextTransition + Send + Sync> FeatureState for OptionalBreakState<BA, JT> {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        process_conditional_jobs(solution_ctx, Some(route_index), &self.context_transition);
    }

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        process_conditional_jobs(solution_ctx, None, &self.context_transition);
        remove_invalid_breaks(solution_ctx, &self.aspects);
    }

    fn state_keys(&self) -> Iter<StateKey> {
        [].iter()
    }
}

/// Mark job as ignored only if it has a break type and vehicle id is not present in routes
fn is_required_job<BA: BreakAspects>(
    aspects: &BA,
    routes: &[RouteContext],
    route_index: Option<usize>,
    job: &Job,
    default: bool,
) -> bool {
    let candidate = BreakCandidate::Job(job);
    if !aspects.is_break_job(candidate) {
        return default;
    }

    if let Some(route_index) = route_index {
        can_be_scheduled(aspects, &routes[route_index], candidate)
    } else {
        routes.iter().any(|route_ctx| {
            aspects.belongs_to_route(route_ctx, candidate) && can_be_scheduled(aspects, route_ctx, candidate)
        })
    }
}

/// Removes breaks which conditions are violated after ruin:
/// * break without location served separately when original job is removed, but break is kept.
/// * break is defined by interval, but its time is violated. This might happen due to departure time rescheduling.
fn remove_invalid_breaks<BA: BreakAspects>(solution_ctx: &mut SolutionContext, aspects: &BA) {
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
                        .filter(|single| aspects.is_break_job(BreakCandidate::Single(single.as_ref())))
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

                    let is_orphan = prev != current && break_single.places.first().and_then(|p| p.location).is_none();
                    let is_not_on_time = !is_on_proper_time(route_ctx, break_single, &activity.schedule)
                        || !can_be_scheduled(aspects, route_ctx, BreakCandidate::Single(break_single));
                    let is_ovrp_last = route_ctx.route().tour.end().map_or(false, |end| std::ptr::eq(activity, end));

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

    // NOTE remove stale breaks from violation list
    solution_ctx.ignored.extend(
        solution_ctx
            .unassigned
            .extract_if({
                let routes = solution_ctx.routes.as_slice();
                move |job, _| !is_required_job(aspects, routes, None, job, true)
            })
            .map(|(job, _)| job),
    );
}

/// Checks whether break can be scheduled in route.
fn can_be_scheduled<BA: BreakAspects>(aspects: &BA, route_ctx: &RouteContext, candidate: BreakCandidate<'_>) -> bool {
    let departure = route_ctx.route().tour.start().unwrap().schedule.departure;
    let arrival = route_ctx.route().tour.end().map_or(0., |end| end.schedule.arrival);
    let tour_tw = TimeWindow::new(departure, arrival);
    let policy = aspects.get_policy(candidate).unwrap_or(BreakPolicy::SkipIfNoIntersection);

    let break_single = candidate.as_single().expect("break job must be a single job");

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
