#[cfg(test)]
#[path = "../../../tests/unit/construction/features/fast_service_test.rs"]
mod fast_service_test;

use super::*;
use crate::construction::enablers::{calculate_travel, calculate_travel_delta, RouteIntervals};
use hashbrown::HashMap;
use std::marker::PhantomData;

/// Creates a feature to prefer a fast serving of jobs.
pub fn create_fast_service_feature<T: LoadOps>(
    name: &str,
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
    state_key: StateKey,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(FastServiceObjective::<T>::new(transport, activity, route_intervals, state_key))
        .with_state(FastServiceState::new(state_key))
        .build()
}

/// Defines how time interval should be calculated for different types of the jobs specified by their
/// demand type.
enum TimeIntervalType {
    /// Time is counted from the start of the start (or latest reload). Corresponds to static
    /// delivery demand jobs.
    FromStart,
    /// Time is counted till the end of the tour (or next reload). Corresponds to static pickup jobs.
    ToEnd,
    /// Time is counted from first activity to the last of the job. Corresponds to dynamic demand jobs.
    FromFirstToLast,
    /// Time is counted from start to the end of the tour (or previous and next reload).
    /// Corresponds to static pickup/delivery jobs.
    FromStartToEnd,
}

/// Keeps track of first and last activity in the tour for specific multi job.
type MultiJobRanges = HashMap<Job, (usize, usize)>;

struct FastServiceObjective<T> {
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
    state_key: StateKey,
    phantom: PhantomData<T>,
}

impl<T: LoadOps> Objective for FastServiceObjective<T> {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| {
                route_ctx.route().tour.jobs().filter(|job| !self.route_intervals.is_marker_job(job)).map(
                    |job| match job {
                        Job::Single(_) => self.estimate_single_job(route_ctx, job),
                        Job::Multi(_) => self.estimate_multi_job(route_ctx, job),
                    },
                )
            })
            .sum::<Cost>()
    }
}

impl<T: LoadOps> FeatureObjective for FastServiceObjective<T> {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        let (route_ctx, activity_ctx) = match move_ctx {
            MoveContext::Route { .. } => return Cost::default(),
            MoveContext::Activity { route_ctx, activity_ctx } => (route_ctx, activity_ctx),
        };

        let activity_idx = activity_ctx.index;

        let (single, job) =
            if let Some((single, job)) = activity_ctx.target.job.as_ref().zip(activity_ctx.target.retrieve_job()) {
                (single, job)
            } else {
                return self.get_departure(route_ctx, activity_ctx) - self.get_start_time(route_ctx, activity_idx);
            };

        // NOTE: for simplicity, we ignore impact on already inserted jobs on local objective level
        match get_time_interval_type::<T>(&job, single.as_ref()) {
            TimeIntervalType::FromStart => {
                self.get_departure(route_ctx, activity_ctx) - self.get_start_time(route_ctx, activity_idx)
            }
            TimeIntervalType::ToEnd => {
                let departure = self.get_departure(route_ctx, activity_ctx);
                let (_, duration_delta) = calculate_travel_delta(route_ctx, activity_ctx, self.transport.as_ref());

                self.get_end_time(route_ctx, activity_idx) + duration_delta - departure
            }
            TimeIntervalType::FromFirstToLast => self.get_cost_for_multi_job(route_ctx, activity_ctx),
            TimeIntervalType::FromStartToEnd => {
                self.get_end_time(route_ctx, activity_idx) - self.get_start_time(route_ctx, activity_idx)
            }
        }
    }
}

impl<T: LoadOps> FastServiceObjective<T> {
    fn new(
        transport: Arc<dyn TransportCost + Send + Sync>,
        activity: Arc<dyn ActivityCost + Send + Sync>,
        route_intervals: Arc<dyn RouteIntervals + Send + Sync>,
        state_key: StateKey,
    ) -> Self {
        Self { transport, activity, route_intervals, state_key, phantom: Default::default() }
    }

    fn get_start_time(&self, route_ctx: &RouteContext, activity_idx: usize) -> Timestamp {
        let (start_idx, _) = self.get_route_interval(route_ctx, activity_idx);
        route_ctx.route().tour[start_idx].schedule.departure
    }

    fn get_end_time(&self, route_ctx: &RouteContext, activity_idx: usize) -> Timestamp {
        let (_, end_idx) = self.get_route_interval(route_ctx, activity_idx);
        route_ctx.route().tour[end_idx].schedule.arrival
    }

    fn get_departure(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> Timestamp {
        // TODO optimize: clients are interested also in travel delta, so we can do needed calculations once
        //      and avoid `calculate_travel_delta` call later
        let (_, (prev_to_tar_dur, _)) = calculate_travel(route_ctx, activity_ctx, self.transport.as_ref());
        let arrival = activity_ctx.prev.schedule.departure + prev_to_tar_dur;

        self.activity.estimate_departure(route_ctx.route(), activity_ctx.target, arrival)
    }

    fn get_cost_for_multi_job(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> Cost {
        let departure = self.get_departure(route_ctx, activity_ctx);
        let range = route_ctx
            .state()
            .get_route_state::<MultiJobRanges>(self.state_key)
            .zip(activity_ctx.target.retrieve_job())
            .and_then(|(jobs, job)| jobs.get(&job))
            .copied();

        let (start_idx, end_idx) = if let Some(range) = range {
            (range.0, range.1)
        } else {
            return departure - self.get_start_time(route_ctx, activity_ctx.index);
        };

        let (_, duration_delta) = calculate_travel_delta(route_ctx, activity_ctx, self.transport.as_ref());

        // NOTE ignore impact of insertion
        match (start_idx, activity_ctx.index, end_idx) {
            (start_idx, activity_idx, end_idx) if activity_idx <= start_idx => {
                route_ctx.route().tour[end_idx].schedule.departure - departure + duration_delta
            }
            (start_idx, activity_idx, end_idx) if activity_idx >= end_idx => {
                departure - route_ctx.route().tour[start_idx].schedule.departure + duration_delta
            }
            _ => Cost::default(),
        }
    }

    fn get_route_interval(&self, route_ctx: &RouteContext, activity_idx: usize) -> (usize, usize) {
        let last_idx = (route_ctx.route().tour.total() as i32 - 1).max(0) as usize;
        self.route_intervals
            .get_marker_intervals(route_ctx)
            .and_then(|intervals| intervals.iter().find(|(start, end)| *start <= activity_idx && *end > activity_idx))
            .copied()
            .unwrap_or((0, last_idx))
    }

    fn estimate_single_job(&self, route_ctx: &RouteContext, job: &Job) -> Cost {
        let single = job.to_single();
        let tour = &route_ctx.route().tour;
        let activity_idx = tour.index(job).expect("cannot find index for job");
        let activity = &tour[activity_idx];

        (match get_time_interval_type::<T>(job, single) {
            TimeIntervalType::FromStart => activity.schedule.departure - self.get_start_time(route_ctx, activity_idx),
            TimeIntervalType::ToEnd => self.get_end_time(route_ctx, activity_idx) - activity.schedule.departure,
            TimeIntervalType::FromStartToEnd => {
                self.get_end_time(route_ctx, activity_idx) - self.get_start_time(route_ctx, activity_idx)
            }
            TimeIntervalType::FromFirstToLast => unreachable!("this type is only for multi job"),
        }) as Cost
    }

    fn estimate_multi_job(&self, route_ctx: &RouteContext, job: &Job) -> Cost {
        route_ctx
            .state()
            .get_route_state::<MultiJobRanges>(self.state_key)
            .and_then(|job_ranges| job_ranges.get(job))
            .map(|&(start_idx, end_idx)| {
                self.get_end_time(route_ctx, end_idx) - self.get_start_time(route_ctx, start_idx)
            })
            .unwrap_or_default()
    }
}

struct FastServiceState {
    state_keys: [StateKey; 1],
}

impl FastServiceState {
    pub fn new(state_key: StateKey) -> Self {
        Self { state_keys: [state_key] }
    }
}

impl FeatureState for FastServiceState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        self.accept_route_state(&mut solution_ctx.routes[route_index]);
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        // keep track of [start, end] positions of all multi jobs in the given tour
        let multi_job_ranges: MultiJobRanges = route_ctx
            .route()
            .tour
            .jobs()
            .filter(|job| job.as_multi().is_some())
            .map(|job| {
                let tour = &route_ctx.route().tour;
                let start_idx = tour.index(job).expect("job start index cannot be found");
                let end_idx = tour.index_last(job).expect("job end index cannot be found");

                (job.clone(), (start_idx, end_idx))
            })
            .collect();

        if !multi_job_ranges.is_empty() {
            route_ctx.state_mut().put_route_state(self.state_keys[0], multi_job_ranges);
        }
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx
            .routes
            .iter_mut()
            .filter(|route_ctx| route_ctx.is_stale())
            .for_each(|route_ctx| self.accept_route_state(route_ctx))
    }

    fn state_keys(&self) -> Iter<StateKey> {
        self.state_keys.iter()
    }
}

fn get_time_interval_type<T: LoadOps>(job: &Job, single: &Single) -> TimeIntervalType {
    if job.as_multi().is_some() {
        return TimeIntervalType::FromFirstToLast;
    }

    let demand: &Demand<T> =
        if let Some(demand) = single.dimens.get_demand() { demand } else { return TimeIntervalType::FromStart };

    match (demand.delivery.0.is_not_empty(), demand.pickup.0.is_not_empty()) {
        (true, false) => TimeIntervalType::FromStart,
        (false, true) => TimeIntervalType::ToEnd,
        _ => TimeIntervalType::FromStartToEnd,
    }
}
