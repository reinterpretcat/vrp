//! A features to put some extra limits on tour.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/tour_limits_test.rs"]
mod tour_limits_test;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::iter::once;
use std::ops::ControlFlow;

use super::*;
use crate::construction::enablers::*;
use crate::models::common::{Distance, Duration, Timestamp};
use crate::models::problem::{Actor, Single, TransportCost};
use crate::models::solution::Tour;

/// A function which returns activity size limit for a given actor.
pub type ActivitySizeResolver = Arc<dyn Fn(&Actor) -> Option<usize> + Sync + Send>;
/// A function which tells whether a job's activity counts toward the tour size. A break, a reload
/// or a recharge sits on the tour as an activity too, but it is not a stop.
pub type ActivityCountFn = Arc<dyn Fn(&Single) -> bool + Sync + Send>;
/// A function to resolve travel limit.
pub type TravelLimitFn<T> = Arc<dyn Fn(&Actor) -> Option<T> + Send + Sync>;

/// Creates a limit for activity amount in a tour.
/// This is a hard constraint.
pub fn create_activity_limit_feature(
    name: &str,
    code: ViolationCode,
    limit_func: ActivitySizeResolver,
    is_counted: ActivityCountFn,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(ActivityLimitConstraint { code, limit_fn: limit_func, is_counted })
        .build()
}

/// Creates a minimum limit for activity amount in a tour.
/// This is a soft constraint (objective) that penalizes solutions where routes have fewer activities than the minimum.
/// Routes with zero activities (empty routes) are allowed.
/// The penalty helps guide the solver toward solutions that meet the minimum, while still allowing
/// exploration of solutions that don't meet the minimum during the search.
pub fn create_min_activity_limit_feature(
    name: &str,
    min_limit_fn: ActivitySizeResolver,
    is_counted: ActivityCountFn,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(MinActivityLimitObjective { min_limit_fn, is_counted })
        .build()
}

/// The activities on the tour that count toward its size.
fn counted_activities(tour: &Tour, is_counted: &ActivityCountFn) -> usize {
    tour.all_activities().filter(|activity| activity.job.as_ref().is_some_and(|single| is_counted(single))).count()
}

/// The activities a job would add to the count once inserted.
fn counted_job_activities(job: &Job, is_counted: &ActivityCountFn) -> usize {
    match job {
        Job::Single(single) => usize::from(is_counted(single)),
        Job::Multi(multi) => multi.jobs.iter().filter(|single| is_counted(single)).count(),
    }
}

/// Creates a travel limits such as distance and/or duration.
/// This is a hard constraint.
///
/// `reserved_times_index` names the actors whose shift carries reserved time (a required break).
/// Pass an empty index when the problem has none: the duration limit then keeps its cheap estimate
/// and no route pays for the lookup.
pub fn create_travel_limit_feature(
    name: &str,
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
    distance_code: ViolationCode,
    duration_code: ViolationCode,
    tour_distance_limit_fn: TravelLimitFn<Distance>,
    tour_duration_limit_fn: TravelLimitFn<Duration>,
    reserved_times_index: ReservedTimesIndex,
) -> Result<Feature, GenericError> {
    // only the total per actor is ever needed, and it bounds what the replay can add
    let reserved_time_totals = reserved_times_index
        .iter()
        .map(|(actor, spans)| (actor.clone(), spans.iter().map(|span| span.duration).sum::<Duration>()))
        .collect::<HashMap<_, _>>();

    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(TravelLimitConstraint {
            transport: transport.clone(),
            activity: activity.clone(),
            tour_distance_limit_fn,
            tour_duration_limit_fn: tour_duration_limit_fn.clone(),
            distance_code,
            duration_code,
            reserved_time_totals: (!reserved_time_totals.is_empty()).then_some(reserved_time_totals),
        })
        .with_state(TravelLimitState { tour_duration_limit_fn, transport, activity })
        .build()
}

struct ActivityLimitConstraint {
    code: ViolationCode,
    limit_fn: ActivitySizeResolver,
    is_counted: ActivityCountFn,
}

impl FeatureConstraint for ActivityLimitConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => {
                (self.limit_fn)(route_ctx.route().actor.as_ref()).and_then(|limit| {
                    let tour_activities = counted_activities(&route_ctx.route().tour, &self.is_counted);
                    let job_activities = counted_job_activities(job, &self.is_counted);

                    if tour_activities + job_activities > limit {
                        ConstraintViolation::fail(self.code)
                    } else {
                        ConstraintViolation::success()
                    }
                })
            }
            MoveContext::Activity { .. } => ConstraintViolation::success(),
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        Ok(source)
    }
}

/// Objective that penalizes routes with fewer activities than the minimum limit.
/// This guides the solver toward valid solutions while still allowing exploration.
struct MinActivityLimitObjective {
    min_limit_fn: ActivitySizeResolver,
    is_counted: ActivityCountFn,
}

/// Penalty carried by a single route for staying below the minimum activity count.
///
/// Squared rather than linear on purpose. With a linear penalty the objective is a plain sum of
/// deficits, which makes it blind to redistribution: moving a job from one under-sized route to
/// another lowers one deficit by 1 and raises the other by 1, so the sum does not move and the
/// search has no reason to consolidate. Squaring makes the same move an improvement whenever it
/// evens the routes out, which is the behaviour the limit is meant to express.
///
/// Empty routes are not penalized — they carry no tour at all.
fn min_activity_penalty(activity_count: usize, min_limit: usize) -> Cost {
    if activity_count == 0 || activity_count >= min_limit {
        return Cost::default();
    }

    let deficit = (min_limit - activity_count) as Cost;

    deficit * deficit
}

impl FeatureObjective for MinActivityLimitObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution.solution.routes.iter().fold(0., |acc, route_ctx| {
            match (self.min_limit_fn)(route_ctx.route().actor.as_ref()) {
                Some(min_limit) => {
                    acc + min_activity_penalty(counted_activities(&route_ctx.route().tour, &self.is_counted), min_limit)
                }
                None => acc,
            }
        })
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        // Report the marginal penalty change of putting this job into this route, so that a route
        // which is still below the minimum is preferred over one that is not — and over opening yet
        // another route. Returning a constant here (as before) left the layer without any influence
        // on route choice at all.
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => (self.min_limit_fn)(route_ctx.route().actor.as_ref())
                .map(|min_limit| {
                    let current = counted_activities(&route_ctx.route().tour, &self.is_counted);
                    let added = counted_job_activities(job, &self.is_counted);

                    min_activity_penalty(current + added, min_limit) - min_activity_penalty(current, min_limit)
                })
                .unwrap_or_default(),
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct TravelLimitConstraint {
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
    tour_distance_limit_fn: TravelLimitFn<Distance>,
    tour_duration_limit_fn: TravelLimitFn<Duration>,
    distance_code: ViolationCode,
    duration_code: ViolationCode,
    /// Total reserved time per actor whose shift carries any, or `None` when the problem declares
    /// none at all.
    reserved_time_totals: Option<HashMap<Arc<Actor>, Duration>>,
}

impl TravelLimitConstraint {
    fn calculate_travel(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> (Distance, Duration) {
        calculate_travel_delta(route_ctx, activity_ctx, self.transport.as_ref())
    }

    /// Returns the duration the tour would have with the target activity in it.
    ///
    /// The travel delta prices the legs the insertion touches and assumes the rest of the tour is
    /// merely pushed along by it. That holds while every second of the tour is either travel,
    /// waiting or service — but not when the shift carries reserved time. A required break is not a
    /// job: it is a charge `DynamicActivityCost` and `DynamicTransportCost` apply wherever an
    /// activity's schedule or a travel leg happens to run into its window, so what it costs depends
    /// on the schedule the insertion produces. The delta prices the target with its bare service
    /// duration and never revisits the activities behind it, so a break swallowed by the target's
    /// own service — or reached by a later activity once the insertion pushes it into the window —
    /// lands in the tour's duration without ever being weighed against the limit.
    ///
    /// For those actors the duration is therefore replayed rather than estimated. So is the one case
    /// where the delta is blind to the measurement itself rather than to what fills the tour — an
    /// insertion that moves a `FirstJobTo*` span's anchor, which needs no break to go wrong. Everyone
    /// else keeps the delta, and pays a single index comparison for this.
    fn calculate_total_duration(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
        change_duration: Duration,
        duration_limit: Duration,
    ) -> Duration {
        let delta_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.) + change_duration;
        let is_delta_blind_to_span = self.is_delta_blind_to_span(route_ctx, activity_ctx);

        let Some(reserved_total) = self.reserved_time_total(route_ctx) else {
            return if is_delta_blind_to_span {
                self.replay_total_duration(route_ctx, activity_ctx, duration_limit)
            } else {
                delta_duration - self.reclaimable_leading_wait(route_ctx, activity_ctx)
            };
        };

        // The walk cannot come out above this: the delta already over-states how far the insertion
        // pushes the tour's end (it assumes nothing downstream absorbs it), and every second the walk
        // can add on top of that is reserved time, of which the shift has `reserved_total`. So a tour
        // that fits even with all of it charged cannot be refused by the walk, and does not need it.
        // One addition buys that for every route with room to spare, which is what a problem that
        // puts a break on every vehicle would feel rather than the 11 of 378 shifts here.
        //
        // NOTE the bound is taken against the delta *without* `reclaimable_leading_wait`. A walk that
        // has to give the idle stretch back — the departure the route wanted turns out infeasible or
        // over the cap — lands above the reclaimed figure by exactly that stretch.
        if !is_delta_blind_to_span && delta_duration + reserved_total <= duration_limit {
            return delta_duration;
        }

        self.replay_total_duration(route_ctx, activity_ctx, duration_limit)
    }

    /// The reserved time this route's actor carries in total, or `None` when it carries none.
    fn reserved_time_total(&self, route_ctx: &RouteContext) -> Option<Duration> {
        self.reserved_time_totals.as_ref().and_then(|totals| totals.get(&route_ctx.route().actor).copied())
    }

    /// Whether the travel delta misses a part of what this insertion does to the tour's *measured*
    /// duration — not to what fills the tour, but to where the measurement starts.
    ///
    /// `calculate_travel_delta` prices legs; it knows nothing of the shift's `RouteCostSpan`. A
    /// `FirstJobTo*` span is measured from the first job's arrival (`calculate_route_duration`), and
    /// an insertion in front of the first job moves that anchor earlier — by the whole difference
    /// between the two depot legs, which the delta never reports. The tour can then run hours longer
    /// than the delta says while the limit reads it as having room to spare.
    ///
    /// This needs no reserved time to go wrong, so both paths consult it: the walk handles the moved
    /// anchor (`replay_tail`), the delta cannot, and the dominance guard has no bound to stand on
    /// while it holds. It is asked index-first so that a shift which is not in this case pays one
    /// integer comparison per position and reads the span on the single leg at index 0.
    fn is_delta_blind_to_span(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> bool {
        activity_ctx.index == 0
            && matches!(
                route_ctx.route().actor.vehicle.dimens.get_route_cost_span().copied().unwrap_or_default(),
                RouteCostSpan::FirstJobToDepot | RouteCostSpan::FirstJobToLastJob
            )
    }

    /// Replays the tour behind the insertion point to get the duration the route would really have.
    ///
    /// This is the forward pass `update_schedules` runs when the route state is accepted, restricted
    /// to the stretch the insertion can move: the activities in front of the target keep the
    /// schedules they already have. It goes through the same activity and transport costs, so it
    /// charges reserved time exactly where the accepted route will be charged for it.
    ///
    /// That holds for a route whose shift carries no reserved time too, which is what lets the
    /// non-reserved path use the walk for a moved `FirstJobTo*` anchor. Do not read that as the costs
    /// being the plain ones there: whether to wrap them in `DynamicActivityCost` /
    /// `DynamicTransportCost` is decided once for the whole problem from a non-empty
    /// `reserved_times_index`, so in a mixed fleet every actor carries the wrapped costs, those with
    /// no break of their own included. What degenerates is the wrapping, not the choice —
    /// `reserved_times_fn` finds no entry for such an actor, returns `None`, and leaves the plain
    /// arithmetic underneath.
    ///
    /// A route that is still empty is replayed from the departure it will move to rather than the
    /// shift's `earliest` it currently sits at — `apply_insertion` advances a new route's departure
    /// the moment the first job goes in. That is the same stretch `reclaimable_leading_wait`
    /// reclaims, taken at the front instead of subtracted at the end, which matters here because a
    /// reserved window is charged differently depending on whether it falls into idling or into
    /// work: leaving later can drop a break out of the tour altogether, or move it out of the idle
    /// and into the service where it is charged in full.
    ///
    /// The later departure is only taken on the same terms `advance_departure_time` takes it: the
    /// schedule it produces has to hold together, and the tour it produces has to fit the cap.
    /// Reserved time is a common reason for neither to hold — a break that lands in the service
    /// instead of the idle can push work past the end of its own window, which `DynamicActivityCost`
    /// refuses, and it is charged in full where the idle used to cover part of it. The departure then
    /// goes straight back and the tour keeps the whole idle stretch inside its duration, which is
    /// exactly the shape the cap has to be read against.
    fn replay_total_duration(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
        duration_limit: Duration,
    ) -> Duration {
        let leading_wait = self.reclaimable_leading_wait(route_ctx, activity_ctx);

        if leading_wait > 0. {
            let (duration, is_feasible) = self.replay_tail(route_ctx, activity_ctx, leading_wait);
            if is_feasible && duration <= duration_limit {
                return duration;
            }
        }

        self.replay_tail(route_ctx, activity_ctx, Duration::default()).0
    }

    /// Walks the tail once from a departure shifted by `departure_shift`, and reports the duration
    /// the route would then have together with whether that schedule is feasible at all.
    ///
    /// Limitation: reserved time carried as an offset span (`VehicleRequiredBreakTime::OffsetTime`)
    /// is resolved by `DynamicActivityCost` against `get_offset_anchor(route)`, which reads the
    /// route as it stands and therefore does not move with `departure_shift`. A shifted walk can
    /// thus price such a break where the real route, once its departure has moved, would re-anchor
    /// it elsewhere. Only the shifted walk is affected, so only the first job of a tour on a shift
    /// whose breaks are offset-based; a break given as `ExactTime` has a fixed window and is exact.
    fn replay_tail(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
        departure_shift: Duration,
    ) -> (Duration, bool) {
        let route = route_ctx.route();
        let tail = once(activity_ctx.target).chain(route.tour.all_activities().skip(activity_ctx.index + 1));

        let mut location = activity_ctx.prev.place.location;
        let mut departure = activity_ctx.prev.schedule.departure + departure_shift;
        let mut target_arrival = Timestamp::default();
        let mut last_job_departure = None;
        let mut is_feasible = true;

        for (offset, activity) in tail.enumerate() {
            let travel =
                self.transport.duration(route, location, activity.place.location, TravelTime::Departure(departure));
            let arrival = departure + travel;

            location = activity.place.location;
            departure = match self.activity.estimate_departure(route, activity, arrival) {
                ControlFlow::Continue(departure) => departure,
                ControlFlow::Break(departure) => {
                    is_feasible = false;
                    departure
                }
            };

            if offset == 0 {
                target_arrival = arrival;
            }
            if activity.job.is_some() {
                last_job_departure = Some(departure);
            }
        }

        // `departure_shift` is zero unless the route is still empty, in which case `prev` is the start
        let start_departure =
            route.tour.start().map_or(Timestamp::default(), |start| start.schedule.departure) + departure_shift;
        // the target takes over as the first job only when it goes in right behind the start
        let first_job_arrival = if activity_ctx.index == 0 {
            target_arrival
        } else {
            route.tour.get(1).map_or(start_departure, |first_job| first_job.schedule.arrival)
        };

        // mirrors `calculate_route_duration`: the span says which part of the tour the route is
        // charged for, and the limit must be read against the same stretch
        let duration = match route.actor.vehicle.dimens.get_route_cost_span().copied().unwrap_or_default() {
            RouteCostSpan::DepotToDepot => departure - start_departure,
            RouteCostSpan::DepotToLastJob => last_job_departure.unwrap_or(start_departure) - start_departure,
            RouteCostSpan::FirstJobToDepot => departure - first_job_arrival,
            RouteCostSpan::FirstJobToLastJob => last_job_departure.unwrap_or(first_job_arrival) - first_job_arrival,
        };

        (duration, is_feasible)
    }

    /// Returns the idle stretch in front of the first job of an otherwise empty route, which the
    /// duration limit must not be charged for.
    ///
    /// An empty route departs at its shift's `earliest`, and `advance_departure_time` bails out on
    /// a tour without jobs (`departure_time.rs`), so it cannot run during insertion evaluation.
    /// The travel delta, however, already contains the waiting time (`calculate_travel_leg`), so a
    /// full-day shift charges everything between the shift start and the job's time window against
    /// the limit — and a job whose window opens late in the day can then never open a tour, even
    /// though the route it would form is trivially short.
    ///
    /// Reclaiming that stretch is not a relaxation: moving the departure forward by it is always
    /// legal on a virgin route, because there is no other activity whose schedule could break, and
    /// a break (`TimeSpan::Offset`) cannot be present either. Routes that already carry a job are
    /// left alone — there the shift is bounded by the existing activities.
    ///
    /// A shift with reserved time keeps the same right to open a tour with a late time window: the
    /// replay departs this much later instead of subtracting the stretch afterwards, which is the
    /// same relief and gets the break charge right on top of it.
    fn reclaimable_leading_wait(&self, route_ctx: &RouteContext, activity_ctx: &ActivityContext) -> Duration {
        if route_ctx.route().tour.job_count() != 0 {
            return Duration::default();
        }

        let route = route_ctx.route();
        let prev = activity_ctx.prev;
        let departure = prev.schedule.departure;

        let travel = self.transport.duration(
            route,
            prev.place.location,
            activity_ctx.target.place.location,
            TravelTime::Departure(departure),
        );

        let latest_departure =
            route.actor.detail.start.as_ref().and_then(|start| start.time.latest).unwrap_or(Float::MAX);

        (activity_ctx.target.place.time.start - departure - travel).max(0.).min(latest_departure - departure)
    }
}

impl FeatureConstraint for TravelLimitConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { .. } => None,
            MoveContext::Activity { route_ctx, activity_ctx, .. } => {
                let tour_distance_limit = (self.tour_distance_limit_fn)(route_ctx.route().actor.as_ref());
                let tour_duration_limit = (self.tour_duration_limit_fn)(route_ctx.route().actor.as_ref());

                if tour_distance_limit.is_some() || tour_duration_limit.is_some() {
                    let (change_distance, change_duration) = self.calculate_travel(route_ctx, activity_ctx);

                    if let Some(distance_limit) = tour_distance_limit {
                        let curr_dis = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
                        let total_distance = curr_dis + change_distance;
                        if distance_limit < total_distance {
                            return ConstraintViolation::skip(self.distance_code);
                        }
                    }

                    if let Some(duration_limit) = tour_duration_limit {
                        let total_duration =
                            self.calculate_total_duration(route_ctx, activity_ctx, change_duration, duration_limit);
                        if duration_limit < total_duration {
                            return ConstraintViolation::skip(self.duration_code);
                        }
                    }
                }

                None
            }
        }
    }

    fn merge(&self, source: Job, _: Job) -> Result<Job, ViolationCode> {
        Ok(source)
    }
}

struct TravelLimitState {
    tour_duration_limit_fn: TravelLimitFn<Duration>,
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
}

impl FeatureState for TravelLimitState {
    fn notify_failure(&self, solution_ctx: &mut SolutionContext, route_indices: &[usize], jobs: &[Job]) -> bool {
        let has_empty_routes_with_limit = route_indices
            .iter()
            .filter(|&&idx| solution_ctx.routes[idx].state().get_limit_duration().is_some())
            .any(|&idx| solution_ctx.routes[idx].route().tour.job_count() == 0);

        // skip if we already have empty routes with limit to prevent the algorithm to stuck
        if has_empty_routes_with_limit {
            return false;
        }

        // find an available actor with duration limits
        let Some((route, actor, start_place)) = solution_ctx
            .registry
            .next_route()
            .filter(|route_ctx| (self.tour_duration_limit_fn)(route_ctx.route().actor.as_ref()).is_some())
            .map(|route_ctx| route_ctx.route())
            .filter_map(|route| route.actor.detail.start.clone().map(|start| (route, route.actor.clone(), start)))
            .next()
        else {
            return false;
        };

        // find departure time for a job that could potentially be served
        // NOTE: assume that jobs are reshuffled time to time to avoid bias.
        let Some(new_departure_time) = jobs
            .iter()
            .flat_map(|job| job.places())
            .filter_map(|place| {
                place.location.map(|location| {
                    place
                        .times
                        .iter()
                        // consider only jobs with time windows
                        .filter_map(|time_span| time_span.as_time_window())
                        // but not max ones
                        .filter(|tw| *tw != TimeWindow::max())
                        .map(move |tw| (tw, location))
                })
            })
            .flatten()
            // filter out jobs which cannot be assigned due to actor's shift time constraint (naive)
            .filter(|(tw, _)| actor.detail.time.contains(tw.start) || actor.detail.time.contains(tw.end))
            .filter_map(|(job_tw, job_loc)| {
                let duration = self.transport.duration_approx(&actor.vehicle.profile, start_place.location, job_loc);

                // consider multiple possible departure times
                [
                    job_tw.end - duration,                                      // latest possible
                    job_tw.start - duration,                                    // earliest possible
                    job_tw.start - duration + (job_tw.end - job_tw.start) / 2., // middle
                ]
                .into_iter()
                // do not depart outside allowed time
                .filter(|&departure_time| {
                    let start_latest = start_place.time.latest.unwrap_or(f64::MAX);
                    let end_latest = actor.detail.end.as_ref().and_then(|place| place.time.latest).unwrap_or(f64::MAX);

                    start_latest.total_cmp(&departure_time) != Ordering::Less
                        && end_latest.total_cmp(&departure_time) != Ordering::Less
                })
                .find(|&departure_time| {
                    // check job can be served with this departure
                    let earliest_departure = start_place.time.earliest.unwrap_or(0.0).max(departure_time);
                    let travel_info = TravelTime::Departure(departure_time);
                    let travel_duration = self.transport.duration(route, start_place.location, job_loc, travel_info);

                    earliest_departure + travel_duration <= job_tw.end
                })
            })
            .next()
        else {
            return false;
        };

        // get route, reschedule it and add to the solution
        let Some(mut route_ctx) = solution_ctx.registry.get_route(&actor) else {
            return false;
        };
        update_route_departure(&mut route_ctx, self.activity.as_ref(), self.transport.as_ref(), new_departure_time);
        solution_ctx.routes.push(route_ctx);

        true
    }

    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        if let Some(limit_duration) = (self.tour_duration_limit_fn)(route_ctx.route().actor.as_ref()) {
            route_ctx.state_mut().set_limit_duration(limit_duration);
        }
    }

    fn accept_solution_state(&self, _: &mut SolutionContext) {}
}
