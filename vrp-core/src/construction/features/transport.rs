//! Provides the way to deal time/distance cost.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/transport_test.rs"]
mod transport_test;

use std::ops::ControlFlow;

use super::*;
use crate::construction::enablers::*;
use crate::models::common::Timestamp;
use crate::models::problem::{
    ActivityCost, OffHoursRateDimension, RegularHoursDimension, Single, TransportCost, TravelTime,
};
use crate::models::solution::Activity;
use rosomaxa::utils::UnwrapValue;

// TODO
//  remove get_total_cost, get_route_costs, get_max_cost methods from contexts
//  add validation rule which ensures usage of only one of these methods.

/// Provides a way to build different flavors of time window feature.
pub struct TransportFeatureBuilder {
    name: String,
    transport: Option<Arc<dyn TransportCost>>,
    activity: Option<Arc<dyn ActivityCost>>,
    code: Option<ViolationCode>,
    is_constrained: bool,
}

impl TransportFeatureBuilder {
    /// Creates a new instance of `TransportFeatureBuilder`.
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), transport: None, activity: None, code: None, is_constrained: true }
    }

    /// Sets constraint violation code which is used to report back the reason of job's unassignment.
    /// If not set, the default violation code is used.
    pub fn set_violation_code(mut self, code: ViolationCode) -> Self {
        self.code = Some(code);
        self
    }

    /// Marks feature as non-constrained meaning that there no need to consider time as a hard constraint.
    /// Default is true.
    pub fn set_time_constrained(mut self, is_constrained: bool) -> Self {
        self.is_constrained = is_constrained;
        self
    }

    /// Sets transport costs to estimate distance.
    pub fn set_transport_cost(mut self, transport: Arc<dyn TransportCost>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Sets activity costs to estimate job start/end time.
    /// If omitted, then [SimpleActivityCost] is used by default.
    pub fn set_activity_cost(mut self, activity: Arc<dyn ActivityCost>) -> Self {
        self.activity = Some(activity);
        self
    }

    /// Builds a flavor of transport feature which only updates activity schedules. No objective, no constraint.
    pub fn build_schedule_updater(mut self) -> GenericResult<Feature> {
        let (transport, activity) = self.get_costs()?;

        FeatureBuilder::default()
            .with_name(self.name.as_str())
            .with_state(TransportState::new(transport, activity))
            .build()
    }

    /// Creates the transport feature which considers duration for minimization as a global objective.
    /// TODO: distance costs are still considered on local level.
    pub fn build_minimize_duration(mut self) -> GenericResult<Feature> {
        let (transport, activity) = self.get_costs()?;

        create_feature(
            self.name.as_str(),
            DurationObjective { transport: transport.clone(), activity: activity.clone() },
            transport,
            activity,
            self.code.unwrap_or_default(),
            self.is_constrained,
        )
    }

    /// Creates the transport feature which considers distance for minimization as a global objective.
    /// TODO: duration costs are still considered on local level.
    pub fn build_minimize_distance(mut self) -> GenericResult<Feature> {
        let (transport, activity) = self.get_costs()?;
        create_feature(
            self.name.as_str(),
            DistanceObjective { transport: transport.clone(), activity: activity.clone() },
            transport,
            activity,
            self.code.unwrap_or_default(),
            self.is_constrained,
        )
    }

    /// Creates the transport feature which considers distance and duration for minimization.
    pub fn build_minimize_cost(mut self) -> GenericResult<Feature> {
        let (transport, activity) = self.get_costs()?;

        create_feature(
            self.name.as_str(),
            CostObjective { transport: transport.clone(), activity: activity.clone() },
            transport,
            activity,
            self.code.unwrap_or_default(),
            self.is_constrained,
        )
    }

    fn get_costs(&mut self) -> GenericResult<(Arc<dyn TransportCost>, Arc<dyn ActivityCost>)> {
        let transport = self.transport.take().ok_or_else(|| GenericError::from("transport must be set"))?;
        let activity = self.activity.take().unwrap_or_else(|| Arc::new(SimpleActivityCost::default()));

        Ok((transport, activity))
    }
}

fn create_feature<O>(
    name: &str,
    objective: O,
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
    time_window_code: ViolationCode,
    is_constrained: bool,
) -> Result<Feature, GenericError>
where
    O: FeatureObjective + 'static,
{
    let builder = FeatureBuilder::default()
        .with_name(name)
        .with_state(TransportState::new(transport.clone(), activity.clone()))
        .with_objective(objective);

    if is_constrained {
        builder
            .with_constraint(TransportConstraint {
                transport: transport.clone(),
                activity: activity.clone(),
                time_window_code,
            })
            .build()
    } else {
        builder.build()
    }
}

struct TransportConstraint {
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
    time_window_code: ViolationCode,
}

impl TransportConstraint {
    fn evaluate_job(&self, route_ctx: &RouteContext, job: &Job) -> Option<ConstraintViolation> {
        let date = route_ctx.route().tour.start().unwrap().schedule.departure;
        let check_single = |single: &Arc<Single>| {
            single
                .places
                .iter()
                .flat_map(|place| place.times.iter())
                .any(|time| time.intersects(date, &route_ctx.route().actor.detail.time))
        };

        let has_time_intersection = match job {
            Job::Single(single) => check_single(single),
            Job::Multi(multi) => multi.jobs.iter().all(check_single),
        };

        if has_time_intersection { None } else { ConstraintViolation::fail(self.time_window_code) }
    }

    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ConstraintViolation> {
        let actor = route_ctx.route().actor.as_ref();
        let route = route_ctx.route();

        let prev = activity_ctx.prev;
        let target = activity_ctx.target;
        let next = activity_ctx.next;

        let departure = prev.schedule.departure;

        if actor.detail.time.end < prev.place.time.start
            || actor.detail.time.end < target.place.time.start
            || next.is_some_and(|next| actor.detail.time.end < next.place.time.start)
        {
            return ConstraintViolation::fail(self.time_window_code);
        }

        let (next_act_location, latest_arr_time_at_next) = if let Some(next) = next {
            let latest_arrival = route_ctx.state().get_latest_arrival_at(activity_ctx.index + 1).copied();
            (next.place.location, latest_arrival.unwrap_or(next.place.time.end))
        } else {
            // open vrp
            (target.place.location, target.place.time.end.min(actor.detail.time.end))
        };

        let arr_time_at_next = departure
            + self.transport.duration(route, prev.place.location, next_act_location, TravelTime::Departure(departure));

        if arr_time_at_next > latest_arr_time_at_next {
            return ConstraintViolation::fail(self.time_window_code);
        }
        if target.place.time.start > latest_arr_time_at_next {
            return ConstraintViolation::skip(self.time_window_code);
        }

        let arr_time_at_target = departure
            + self.transport.duration(
                route,
                prev.place.location,
                target.place.location,
                TravelTime::Departure(departure),
            );

        let latest_departure_at_target = latest_arr_time_at_next
            - self.transport.duration(
                route,
                target.place.location,
                next_act_location,
                TravelTime::Arrival(latest_arr_time_at_next),
            );

        let ControlFlow::Continue(latest_arr_time_at_target) =
            self.activity.estimate_arrival(route, target, latest_departure_at_target)
        else {
            return ConstraintViolation::skip(self.time_window_code);
        };
        let latest_arr_time_at_target = target.place.time.end.min(latest_arr_time_at_target);

        if arr_time_at_target > latest_arr_time_at_target {
            return ConstraintViolation::skip(self.time_window_code);
        }

        if next.is_none() {
            return ConstraintViolation::success();
        }

        let ControlFlow::Continue(end_time_at_target) =
            self.activity.estimate_departure(route, target, arr_time_at_target)
        else {
            return ConstraintViolation::skip(self.time_window_code);
        };

        let arr_time_at_next = end_time_at_target
            + self.transport.duration(
                route,
                target.place.location,
                next_act_location,
                TravelTime::Departure(end_time_at_target),
            );

        if arr_time_at_next > latest_arr_time_at_next {
            ConstraintViolation::skip(self.time_window_code)
        } else {
            ConstraintViolation::success()
        }
    }
}

impl FeatureConstraint for TransportConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.evaluate_job(route_ctx, job),
            MoveContext::Activity { route_ctx, activity_ctx, .. } => self.evaluate_activity(route_ctx, activity_ctx),
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        // NOTE we don't change temporal parameters here, it is responsibility of the caller
        Ok(source)
    }
}

struct DistanceObjective {
    activity: Arc<dyn ActivityCost>,
    transport: Arc<dyn TransportCost>,
}

impl FeatureObjective for DistanceObjective {
    fn fitness(&self, insertion_ctx: &InsertionContext) -> Cost {
        insertion_ctx.solution.routes.iter().fold(Cost::default(), move |acc, route_ctx| {
            acc + route_ctx.state().get_total_distance().copied().unwrap_or(0.)
        })
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        let MoveContext::Activity { route_ctx, activity_ctx, .. } = move_ctx else {
            return Cost::default();
        };

        estimate_leg(self.transport.as_ref(), self.activity.as_ref(), route_ctx, activity_ctx, |from, to, time| {
            self.transport.distance(route_ctx.route(), from, to, time)
        })
    }
}

struct DurationObjective {
    activity: Arc<dyn ActivityCost>,
    transport: Arc<dyn TransportCost>,
}

impl FeatureObjective for DurationObjective {
    fn fitness(&self, insertion_ctx: &InsertionContext) -> Cost {
        insertion_ctx.solution.routes.iter().fold(Cost::default(), move |acc, route_ctx| {
            acc + route_ctx.state().get_total_duration().copied().unwrap_or(0.)
        })
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        let MoveContext::Activity { route_ctx, activity_ctx, .. } = move_ctx else {
            return Cost::default();
        };

        estimate_leg(self.transport.as_ref(), self.activity.as_ref(), route_ctx, activity_ctx, |from, to, time| {
            self.transport.duration(route_ctx.route(), from, to, time)
        })
    }
}

fn estimate_leg<FE>(
    transport: &dyn TransportCost,
    activity: &dyn ActivityCost,
    route_ctx: &RouteContext,
    activity_ctx: &ActivityContext,
    estimate_fn: FE,
) -> Float
where
    FE: Fn(Location, Location, TravelTime) -> Float,
{
    let route = route_ctx.route();
    let prev = activity_ctx.prev.place.location;
    let target = activity_ctx.target.place.location;

    let prev_dep = TravelTime::Departure(activity_ctx.prev.schedule.departure);

    // prev -> target
    let (prev_target, dep_time_target) = {
        let time = activity_ctx.prev.schedule.departure;
        let arrival = time + transport.duration(route, prev, target, prev_dep);
        let departure = activity.estimate_departure(route, activity_ctx.target, arrival).unwrap_value();

        (estimate_fn(prev, target, prev_dep), departure)
    };

    // target -> next
    let target_next = if let Some(next) = activity_ctx.next {
        estimate_fn(target, next.place.location, TravelTime::Departure(dep_time_target))
    } else {
        Distance::default()
    };

    // new transition: prev -> target -> next
    let prev_target_next = prev_target + target_next;

    // no jobs yet or open vrp.
    if !route.tour.has_jobs() {
        return prev_target_next;
    }

    let Some(next) = activity_ctx.next else {
        return prev_target_next;
    };

    // old transition: prev -> next
    let prev_next = estimate_fn(prev, next.place.location, prev_dep);

    // delta change
    prev_target_next - prev_next
}

/// What a tour owes above its regular duration: the difference between the overtime rate and the
/// regular one, charged on the time beyond the threshold.
///
/// The plain time cost already charged that time once, so only the difference on top of it is owed
/// here. A shift which states no regular duration owes nothing, and a rate below the regular one
/// clamps to zero rather than paying the solver to run late.
///
/// NOTE the rate subtracted is the vehicle's `per_driving_time`, while what the tour was actually
/// charged for that second is `max(per_driving, per_service, per_waiting)` for the vehicle plus the
/// same again for the driver. Those agree for every problem the pragmatic reader builds - it sets
/// all three vehicle rates from `costs.time` and leaves the driver's at zero - and only for those.
/// A fleet assembled through the core API with rates which differ, or with a paid driver, has its
/// premium measured against a regular rate it does not pay.
pub fn get_overtime_premium(actor: &Actor, duration: Duration) -> Cost {
    let dimens = &actor.vehicle.dimens;

    let Some(regular) = dimens.get_regular_duration().copied() else {
        return Cost::default();
    };

    let premium = (dimens.get_overtime_rate().copied().unwrap_or(0.) - actor.vehicle.costs.per_driving_time).max(0.);

    premium * (duration - regular).max(0.)
}

/// What a tour owes for the time its paid span lies outside the shift's regular hours: the
/// difference between the off-hours rate and the regular one, on every second before the regular
/// hours start and after they end.
///
/// Built like [`get_overtime_premium`], against the same regular rate and with the same clamp: a
/// shift without regular hours or without a rate owes nothing, and a rate below the regular one
/// never pays the solver to work off hours. It adds to overtime where both apply.
pub fn get_off_hours_premium(actor: &Actor, (from, to): (Timestamp, Timestamp)) -> Cost {
    let dimens = &actor.vehicle.dimens;

    let (Some(hours), Some(rate)) = (dimens.get_regular_hours().copied(), dimens.get_off_hours_rate().copied()) else {
        return Cost::default();
    };

    let premium = (rate - actor.vehicle.costs.per_driving_time).max(0.);
    let outside = (hours.earliest - from).max(0.) + (to - hours.latest).max(0.);

    premium * outside
}

/// What an insertion which grows the tour by `change_duration` adds to the premium it already owes.
///
/// NOTE the new duration is projected as the tour's total plus the change, the same cheap
/// projection `tour_limits` uses in its simplest branch: it ignores a departure move the insertion
/// might cause, and a tour whose duration is measured over a `RouteCostSpan` the change falls
/// outside of. The exact figure is `fitness`'s job; the estimate only has to rank candidates, and a
/// premium which appears one insertion late would let construction fill into overtime blind.
fn get_overtime_premium_delta(route_ctx: &RouteContext, change_duration: Duration) -> Cost {
    let actor = route_ctx.route().actor.as_ref();

    // this runs for every candidate position, and most problems state no regular duration on any
    // shift, so the miss is answered on one dimension lookup and before any state is read.
    if actor.vehicle.dimens.get_regular_duration().is_none() {
        return Cost::default();
    }

    let old_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);
    let new_duration = old_duration + change_duration;

    get_overtime_premium(actor, new_duration) - get_overtime_premium(actor, old_duration)
}

/// What an insertion which pushes the end of the tour back by `change_duration` adds to the off-hours
/// premium: the same tail-side projection as the overtime delta. A start that moves with it (a later
/// departure) is left to `fitness`, as it is for overtime.
fn get_off_hours_premium_delta(route_ctx: &RouteContext, change_duration: Duration) -> Cost {
    let route = route_ctx.route();

    // most problems state no regular hours: answered on one dimension lookup.
    if route.actor.vehicle.dimens.get_regular_hours().is_none() {
        return Cost::default();
    }

    let Some((from, to)) = get_paid_span(route) else {
        return Cost::default();
    };

    get_off_hours_premium(route.actor.as_ref(), (from, to + change_duration))
        - get_off_hours_premium(route.actor.as_ref(), (from, to))
}

/// Both premiums an insertion that grows the tour by `change_duration` adds on top of the time cost.
fn get_time_premium_delta(route_ctx: &RouteContext, change_duration: Duration) -> Cost {
    get_overtime_premium_delta(route_ctx, change_duration) + get_off_hours_premium_delta(route_ctx, change_duration)
}

struct CostObjective {
    activity: Arc<dyn ActivityCost>,
    transport: Arc<dyn TransportCost>,
}

impl CostObjective {
    fn estimate_route(&self, route_ctx: &RouteContext) -> Float {
        if route_ctx.route().tour.has_jobs() {
            0.
        } else {
            route_ctx.route().actor.driver.costs.fixed + route_ctx.route().actor.vehicle.costs.fixed
        }
    }

    fn estimate_activity(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> Float {
        let prev = activity_ctx.prev;
        let target = activity_ctx.target;
        let next = activity_ctx.next;

        let (tp_cost_left, act_cost_left, dep_time_left) =
            self.analyze_route_leg(route_ctx, prev, target, prev.schedule.departure);

        let (tp_cost_right, act_cost_right, dep_time_right) = if let Some(next) = next {
            self.analyze_route_leg(route_ctx, target, next, dep_time_left)
        } else {
            (Cost::default(), Cost::default(), Timestamp::default())
        };

        let new_costs = tp_cost_left + tp_cost_right + act_cost_left + act_cost_right;

        // where the tour's tail ends up with the target in it: `next` pushed along by it, or the
        // target itself once it lands at the end.
        let dep_time_tail = if next.is_some() { dep_time_right } else { dep_time_left };

        // no jobs yet or open vrp: nothing is displaced, so the whole new leg is what the tour grows by.
        if !route_ctx.route().tour.has_jobs() {
            return new_costs + get_time_premium_delta(route_ctx, dep_time_tail - prev.schedule.departure);
        }

        let Some(next) = next else {
            return new_costs + get_time_premium_delta(route_ctx, dep_time_tail - prev.schedule.departure);
        };

        let waiting_time = route_ctx.state().get_waiting_time_at(activity_ctx.index + 1).copied().unwrap_or_default();

        let (tp_cost_old, act_cost_old, dep_time_old) =
            self.analyze_route_leg(route_ctx, prev, next, prev.schedule.departure);

        let waiting_cost = waiting_time.min(Float::default().max(dep_time_right - dep_time_old))
            * route_ctx.route().actor.vehicle.costs.per_waiting_time;

        let old_costs = tp_cost_old + act_cost_old + waiting_cost;

        new_costs - old_costs + get_time_premium_delta(route_ctx, dep_time_tail - dep_time_old)
    }

    fn analyze_route_leg(
        &self,
        route_ctx: &RouteContext,
        start: &Activity,
        end: &Activity,
        time: Timestamp,
    ) -> (Cost, Cost, Timestamp) {
        let route = route_ctx.route();

        let arrival = time
            + self.transport.duration(route, start.place.location, end.place.location, TravelTime::Departure(time));
        let departure = self.activity.estimate_departure(route, end, arrival).unwrap_value();

        let transport_cost =
            self.transport.cost(route, start.place.location, end.place.location, TravelTime::Departure(time));
        let activity_cost = self.activity.cost(route, end, arrival);

        (transport_cost, activity_cost, departure)
    }
}

impl FeatureObjective for CostObjective {
    fn fitness(&self, insertion_ctx: &InsertionContext) -> Cost {
        let base = insertion_ctx.get_total_cost().unwrap_or_default();

        insertion_ctx.solution.routes.iter().fold(base, |acc, route_ctx| {
            let duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

            let actor = route_ctx.route().actor.as_ref();
            let off_hours =
                get_paid_span(route_ctx.route()).map_or(Cost::default(), |span| get_off_hours_premium(actor, span));

            acc + get_overtime_premium(actor, duration) + off_hours
        })
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, .. } => self.estimate_route(route_ctx),
            MoveContext::Activity { route_ctx, activity_ctx, .. } => self.estimate_activity(route_ctx, activity_ctx),
        }
    }
}

struct TransportState {
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
}

impl TransportState {
    fn new(transport: Arc<dyn TransportCost>, activity: Arc<dyn ActivityCost>) -> Self {
        Self { transport, activity }
    }
}

impl FeatureState for TransportState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();
        self.accept_route_state(route_ctx);
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        update_route_schedule(route_ctx, self.activity.as_ref(), self.transport.as_ref());
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx.routes.iter_mut().filter(|route_ctx| route_ctx.is_stale()).for_each(|route_ctx| {
            update_route_schedule(route_ctx, self.activity.as_ref(), self.transport.as_ref());
        })
    }
}
