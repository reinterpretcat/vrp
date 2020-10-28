use crate::construction::heuristics::{InsertionContext, RouteContext};
use crate::models::common::{Duration, TimeSpan, TimeWindow, Timestamp};
use crate::models::problem::Job;
use crate::models::OP_START_MSG;
use crate::solver::mutation::{select_random_route, LocalSearch, Recreate, RecreateWithSkipBest};
use crate::solver::RefinementContext;
use crate::utils::compare_floats;
use std::cmp::Ordering;
use std::iter::once;
use std::sync::Arc;

/// Pushes route departure in the future to reduce waiting time or/and handle more late jobs.
pub struct PushRouteDeparture {
    offset_ratio: f64,
}

impl Default for PushRouteDeparture {
    fn default() -> Self {
        Self::new(0.1)
    }
}

impl PushRouteDeparture {
    /// Creates a new instance of `PushRouteDeparture`.
    pub fn new(offset_ratio: f64) -> Self {
        Self { offset_ratio }
    }
}

impl LocalSearch for PushRouteDeparture {
    fn explore(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        let solution = &insertion_ctx.solution;

        select_random_route(solution.routes.as_slice(), insertion_ctx.random.as_ref(), |route_ctx| {
            route_ctx.route.tour.jobs().all(|job| !solution.locked.contains(&job))
        })
        .and_then(|route_idx| solution.routes.get(route_idx).map(|route_ctx| (route_ctx, route_idx)))
        .and_then(|(route_ctx, route_idx)| {
            get_departure_offset(insertion_ctx, route_ctx, self.offset_ratio).map(|offset| (route_idx, offset))
        })
        .map(|(route_idx, offset)| apply_shift_and_repair(refinement_ctx, insertion_ctx, route_idx, offset))
    }
}

fn get_departure_offset(insertion_ctx: &InsertionContext, route_ctx: &RouteContext, offset_ratio: f64) -> Option<f64> {
    let start = route_ctx.route.tour.start().expect(OP_START_MSG);

    // TODO allow to push it back?
    // already shifted at max
    if compare_floats(start.schedule.departure, get_latest_departure(route_ctx)) == Ordering::Equal {
        return None;
    }

    route_ctx
        .route
        .tour
        .all_activities()
        .last()
        .and_then(|end| get_time_duration(start.schedule.departure, end.schedule.arrival))
        .or_else(|| {
            select_time_window(insertion_ctx).and_then(|time| get_time_duration(start.schedule.departure, time.end))
        })
        .map(|duration| duration * offset_ratio)
}

fn get_time_duration(start: Timestamp, end: Timestamp) -> Option<Duration> {
    if compare_floats(end, std::f64::MAX) == Ordering::Equal {
        None
    } else {
        Some(end - start)
    }
}

fn select_time_window(insertion_ctx: &InsertionContext) -> Option<TimeWindow> {
    // NOTE get random job from jobs list and use its time window
    let jobs = &insertion_ctx.problem.jobs;
    assert!(jobs.size() > 0);

    let skip_idx = insertion_ctx.random.uniform_int(0, (jobs.size() - 1) as i32) as usize;
    jobs.all()
        .skip(skip_idx)
        .filter_map(|job| {
            // NOTE not randomized: we select first single from multi job and first time window from first place
            // TODO construct a vector with unique time windows from the problem definition
            //      at the beginning and then pick a random item here.
            let singles: Box<dyn Iterator<Item = &Arc<_>>> = match &job {
                Job::Single(single) => Box::new(once(single)),
                Job::Multi(multi) => Box::new(multi.jobs.iter()),
            };
            singles
                .filter_map(|single| {
                    single
                        .places
                        .iter()
                        .flat_map(|place| place.times.iter())
                        .filter_map(|span| match span {
                            TimeSpan::Window(time) => Some(time.clone()),
                            _ => None,
                        })
                        .next()
                })
                .next()
        })
        .next()
}

fn apply_shift_and_repair(
    refinement_ctx: &RefinementContext,
    insertion_ctx: &InsertionContext,
    route_idx: usize,
    departure_offset: f64,
) -> InsertionContext {
    let mut new_insertion_ctx = insertion_ctx.deep_copy();
    let mut route_ctx = new_insertion_ctx.solution.routes.get_mut(route_idx).unwrap();

    let jobs = route_ctx.route.tour.jobs().collect::<Vec<_>>();
    jobs.iter().for_each(|job| {
        debug_assert!(!insertion_ctx.solution.locked.contains(job));
        debug_assert!(route_ctx.route_mut().tour.remove(job));
    });
    new_insertion_ctx.solution.required.extend(jobs.into_iter());

    debug_assert!(!route_ctx.route.tour.has_jobs());

    let latest_departure = get_latest_departure(route_ctx);
    let start = route_ctx.route_mut().tour.get_mut(0).expect(OP_START_MSG);
    let new_departure = (start.schedule.arrival + departure_offset).min(latest_departure);

    start.schedule.departure = new_departure;
    new_insertion_ctx.problem.constraint.accept_route_state(&mut route_ctx);

    // NOTE repair solution
    // TODO we can try different recreate methods here
    RecreateWithSkipBest::default().run(refinement_ctx, new_insertion_ctx)
}

fn get_latest_departure(route_ctx: &RouteContext) -> f64 {
    route_ctx.route.actor.detail.start.as_ref().and_then(|s| s.time.latest).unwrap_or(std::f64::MAX)
}
