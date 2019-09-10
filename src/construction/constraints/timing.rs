use crate::construction::constraints::{
    ActivityConstraintViolation, ConstraintModule, ConstraintVariant, HardActivityConstraint,
    HardRouteConstraint, RouteConstraintViolation, SoftActivityConstraint,
};
use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::models::problem::{ActivityCost, Job, TransportCost};
use crate::models::solution::{Route, TourActivity};
use std::cmp::max;
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;

const LATEST_ARRIVAL_KEY: i32 = 1;
const WAITING_KEY: i32 = 2;
const OP_START_MSG: &str = "Optional start is not yet implemented.";

/// Checks whether vehicle can serve activity taking into account their time windows.
/// TODO add extra check that job's and actor's TWs have intersection (hard route constraint).
pub struct TimingConstraintModule {
    code: i32,
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
    activity: Arc<dyn ActivityCost>,
    transport: Arc<dyn TransportCost>,
}

impl ConstraintModule for TimingConstraintModule {
    fn accept_route_state(&self, ctx: &mut RouteContext) {
        let route = ctx.route.read().unwrap();
        let mut state = ctx.state.write().unwrap();
        let start = route.tour.start().unwrap_or(panic!(OP_START_MSG));
        let start = start.read().unwrap();
        let actor = route.actor.as_ref();

        // update each activity schedule
        route.tour.all_activities().skip(1).fold(
            (start.place.location, start.schedule.departure),
            |(loc, dep), activity| {
                let mut a = activity.write().unwrap();

                a.schedule.arrival = dep
                    + self
                        .transport
                        .duration(actor.vehicle.profile, loc, a.place.location, dep);

                a.schedule.departure = a.schedule.arrival.max(a.place.time.start)
                    + self.activity.duration(
                        actor.vehicle.as_ref(),
                        actor.driver.as_ref(),
                        a.deref(),
                        a.schedule.arrival,
                    );

                (a.place.location, a.schedule.departure)
            },
        );

        // update latest arrival and waiting states of non-terminate (jobs) activities
        let init = (
            actor.detail.time.end,
            actor
                .detail
                .end
                .unwrap_or(actor.detail.start.unwrap_or(panic!(OP_START_MSG))),
            0f64,
        );
        route
            .tour
            .all_activities()
            .rev()
            .fold(init, |acc, activity| {
                let act = activity.read().unwrap();
                if act.job.is_none() {
                    return acc;
                }

                let (end_time, prev_loc, waiting) = acc;

                let potential_latest = end_time
                    - self.transport.duration(
                        actor.vehicle.profile,
                        act.place.location,
                        prev_loc,
                        end_time,
                    )
                    - self.activity.duration(
                        actor.vehicle.as_ref(),
                        actor.driver.as_ref(),
                        act.deref(),
                        end_time,
                    );

                let latest_arrival_time = act.place.time.end.min(potential_latest);

                let future_waiting =
                    waiting + (act.place.time.start - act.schedule.arrival).max(0f64);

                state.put_activity_state(LATEST_ARRIVAL_KEY, &activity, latest_arrival_time);
                state.put_activity_state(WAITING_KEY, &activity, future_waiting);

                (latest_arrival_time, act.place.location, future_waiting)
            });
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        // NOTE revise this once routing is sensible to departure time reschedule departure and
        // arrivals if arriving earlier to the first activity do it only in implicit end of algorithm
        if ctx.required.is_empty() {
            ctx.routes
                .iter()
                .filter(|route_ctx| route_ctx.route.read().unwrap().tour.activity_count() > 0)
                .for_each(|route_ctx| reschedule_departure(route_ctx));
        }
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

impl TimingConstraintModule {
    fn new(activity: Arc<dyn ActivityCost>, transport: Arc<dyn TransportCost>, code: i32) -> Self {
        Self {
            code,
            state_keys: vec![LATEST_ARRIVAL_KEY, WAITING_KEY],
            constraints: vec![
                ConstraintVariant::HardActivity(Arc::new(TimeHardActivityConstraint {})),
                ConstraintVariant::SoftActivity(Arc::new(TimeSoftActivityConstraint {})),
            ],
            activity,
            transport,
        }
    }
}

struct TimeHardActivityConstraint {}

impl HardActivityConstraint for TimeHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        unimplemented!()
    }
}

struct TimeSoftActivityConstraint {}

impl SoftActivityConstraint for TimeSoftActivityConstraint {
    fn estimate_activity(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> f64 {
        unimplemented!()
    }
}

fn reschedule_departure(ctx: &RouteContext) {
    unimplemented!()
}
