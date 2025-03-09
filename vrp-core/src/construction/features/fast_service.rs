#[cfg(test)]
#[path = "../../../tests/unit/construction/features/fast_service_test.rs"]
mod fast_service_test;

use super::*;
use crate::construction::enablers::{calculate_travel, calculate_travel_delta};
use std::collections::HashMap;

/// Provides the way to build fast service feature.
pub struct FastServiceFeatureBuilder {
    name: String,
    violation_code: Option<ViolationCode>,
    demand_type_fn: Option<DemandTypeFn>,
    is_filtered_job_fn: Option<IsFilteredJobFn>,
    transport: Option<Arc<dyn TransportCost>>,
    activity: Option<Arc<dyn ActivityCost>>,
}

impl FastServiceFeatureBuilder {
    /// Creates a new instance of `RechargeFeatureBuilder`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            violation_code: None,
            demand_type_fn: None,
            is_filtered_job_fn: None,
            transport: None,
            activity: None,
        }
    }

    /// Sets constraint violation code which is used to report back the reason of job's unassignment.
    pub fn set_violation_code(mut self, violation_code: ViolationCode) -> Self {
        self.violation_code = Some(violation_code);
        self
    }

    /// Sets a function to get job's demand type.
    pub fn set_demand_type_fn<F>(mut self, func: F) -> Self
    where
        F: Fn(&Single) -> Option<DemandType> + Send + Sync + 'static,
    {
        self.demand_type_fn = Some(Arc::new(func));
        self
    }

    /// Sets a function which tells whether the job should NOT be considered for estimation.
    pub fn set_is_filtered_job<F>(mut self, func: F) -> Self
    where
        F: Fn(&Job) -> bool + Send + Sync + 'static,
    {
        self.is_filtered_job_fn = Some(Arc::new(func));
        self
    }

    /// Sets transport costs to estimate distance.
    pub fn set_transport(mut self, transport: Arc<dyn TransportCost>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Sets activity costs to estimate job start/end time.
    pub fn set_activity(mut self, activity: Arc<dyn ActivityCost>) -> Self {
        self.activity = Some(activity);
        self
    }

    /// Builds fast service feature.
    pub fn build(mut self) -> GenericResult<Feature> {
        let transport = self.transport.take().ok_or_else(|| GenericError::from("transport must be set"))?;
        let activity = self.activity.take().ok_or_else(|| GenericError::from("activity must be set"))?;

        let demand_type_fn =
            self.demand_type_fn.take().ok_or_else(|| GenericError::from("demand_type_fn must be set"))?;

        let is_filtered_job_fn = self.is_filtered_job_fn.take().unwrap_or_else(|| Arc::new(|_| false));

        FeatureBuilder::default()
            .with_name(self.name.as_str())
            .with_state(FastServiceState::default())
            .with_objective(FastServiceObjective::new(demand_type_fn, is_filtered_job_fn, transport, activity))
            .build()
    }
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
/// A function to get a demand type from the job.
type DemandTypeFn = Arc<dyn Fn(&Single) -> Option<DemandType> + Send + Sync>;
/// Returns true if job should not be considered for estimation.
type IsFilteredJobFn = Arc<dyn Fn(&Job) -> bool + Send + Sync>;

custom_tour_state!(MultiJobRanges typeof MultiJobRanges);

struct FastServiceObjective {
    demand_type_fn: DemandTypeFn,
    is_filtered_job_fn: IsFilteredJobFn,
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
}

impl FeatureObjective for FastServiceObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| {
                route_ctx.route().tour.jobs().filter(|job| !(self.is_filtered_job_fn)(job)).map(|job| match job {
                    Job::Single(_) => self.estimate_single_job(route_ctx, job),
                    Job::Multi(_) => self.estimate_multi_job(route_ctx, job),
                })
            })
            .sum::<Cost>()
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        let (route_ctx, activity_ctx) = match move_ctx {
            MoveContext::Route { .. } => return Cost::default(),
            MoveContext::Activity { route_ctx, activity_ctx, .. } => (route_ctx, activity_ctx),
        };

        let activity_idx = activity_ctx.index;

        let (single, job) = match activity_ctx.target.job.as_ref().zip(activity_ctx.target.retrieve_job()) {
            Some((single, job)) => (single, job),
            _ => {
                return self.get_departure(route_ctx, activity_ctx) - self.get_start_time(route_ctx, activity_idx);
            }
        };

        // NOTE: for simplicity, we ignore impact on already inserted jobs on local objective level
        match self.get_time_interval_type(&job, single.as_ref()) {
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

impl FastServiceObjective {
    fn new(
        demand_type_fn: DemandTypeFn,
        is_filtered_job_fn: IsFilteredJobFn,
        transport: Arc<dyn TransportCost>,
        activity: Arc<dyn ActivityCost>,
    ) -> Self {
        Self { demand_type_fn, is_filtered_job_fn, transport, activity }
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
            .get_multi_job_ranges()
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

        route_ctx
            .state()
            .get_reload_intervals()
            .and_then(|intervals| intervals.iter().find(|(start, end)| *start <= activity_idx && *end > activity_idx))
            .copied()
            .unwrap_or((0, last_idx))
    }

    fn estimate_single_job(&self, route_ctx: &RouteContext, job: &Job) -> Cost {
        let single = job.to_single();
        let tour = &route_ctx.route().tour;
        let activity_idx = tour.index(job).expect("cannot find index for job");
        let activity = &tour[activity_idx];

        (match self.get_time_interval_type(job, single) {
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
            .get_multi_job_ranges()
            .and_then(|job_ranges| job_ranges.get(job))
            .map(|&(start_idx, end_idx)| {
                self.get_end_time(route_ctx, end_idx) - self.get_start_time(route_ctx, start_idx)
            })
            .unwrap_or_default()
    }

    fn get_time_interval_type(&self, job: &Job, single: &Single) -> TimeIntervalType {
        if job.as_multi().is_some() {
            return TimeIntervalType::FromFirstToLast;
        }

        match (self.demand_type_fn)(single) {
            Some(DemandType::Delivery) => TimeIntervalType::FromStart,
            Some(DemandType::Pickup) => TimeIntervalType::ToEnd,
            Some(_) => TimeIntervalType::FromStartToEnd,
            None => TimeIntervalType::FromStart,
        }
    }
}

#[derive(Default)]
struct FastServiceState {}

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

        // NOTE: always override existing state to avoid stale information about multi-jobs
        route_ctx.state_mut().set_multi_job_ranges(multi_job_ranges);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx
            .routes
            .iter_mut()
            .filter(|route_ctx| route_ctx.is_stale())
            .for_each(|route_ctx| self.accept_route_state(route_ctx))
    }
}
