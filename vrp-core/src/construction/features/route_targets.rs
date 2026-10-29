//! Route targets: every route is scored against targets the caller states per metric, as a
//! piecewise linear curve from the metric's value to penalty points, and the plan minimizes the
//! summed points of its routes.
//!
//! The curve is the caller's grading key: a target "grade A up to 10 km, B up to 20, C up to 30"
//! is the points `(10000, 0), (20000, 1), (30000, 2)`. Between two points the penalty is linear, so
//! the solver has a gradient inside a grade; beyond the outer points the outer segment's slope goes
//! on, so an extreme route is worse than a borderline one; and the result never drops below zero.
//!
//! Distance, driving time and duration are measured over a span of the route (see
//! [`RouteCostSpan`]), by default from the first job to the last one, depot legs left out. A
//! duration leaves the breaks inside its span out unless told to keep them. A target may also be
//! read per job or per hour of work instead of per route.
//!
//! An empty route scores nothing: closing a day removes its points.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/route_targets_test.rs"]
mod route_targets_test;

use super::*;
use crate::models::solution::{Activity, Route};

custom_tour_state!(RouteTargetMetrics typeof RouteMetrics);
custom_solution_state!(RouteTargetsFitness typeof Cost);

/// What a route target measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteMetric {
    /// Travelled distance over the target's span.
    Distance,
    /// Driving time over the target's span.
    DrivingTime,
    /// Elapsed time over the target's span, waiting included, breaks per [`RouteTarget::include_breaks`].
    Duration,
    /// The summed service durations of the route's jobs, breaks not counted.
    ServiceTime,
    /// The summed value of the route's jobs (see [`RouteTargetsBuilder::set_job_value_fn`]).
    Value,
    /// The number of job activities on the route, breaks not counted.
    Activities,
}

/// What a route target's metric is divided by before it is read on the curve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RouteTargetPer {
    /// Nothing: the route's total.
    #[default]
    Route,
    /// The route's number of job activities.
    Activity,
    /// The route's working hours: the time from the first job to the last one, breaks taken out. A
    /// route whose working time is zero scores nothing on such a target.
    Hour,
}

/// How the points of a route's targets combine into the route's score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteTargetAggregate {
    /// Their sum.
    Sum,
    /// The sum of their squares, which leans on the worst one the way a "lowest grade counts" key does.
    SumOfSquares,
    /// The worst one.
    Max,
}

/// One target: a metric and its curve of `(metric value, penalty points)`, ascending by value.
#[derive(Clone, Debug)]
pub struct RouteTarget {
    /// What is measured.
    pub metric: RouteMetric,
    /// The part of the route measured, for distance, driving time and duration.
    pub span: RouteCostSpan,
    /// Whether the breaks inside the span count, for duration.
    pub include_breaks: bool,
    /// What the metric is divided by.
    pub per: RouteTargetPer,
    /// The curve, at least two points, strictly ascending by value.
    pub points: Vec<(Float, Float)>,
}

impl RouteTarget {
    /// Creates a target over the default span (first job to last job), breaks excluded, per route.
    pub fn new(metric: RouteMetric, points: Vec<(Float, Float)>) -> Self {
        Self {
            metric,
            span: RouteCostSpan::FirstJobToLastJob,
            include_breaks: false,
            per: RouteTargetPer::Route,
            points,
        }
    }

    /// Checks the curve: at least two points, finite, strictly ascending by value.
    pub fn validate(&self) -> GenericResult<()> {
        if self.points.len() < 2 {
            return Err(format!("route targets: a curve needs at least two points, got {}", self.points.len()).into());
        }
        if self.points.iter().any(|(value, points)| !value.is_finite() || !points.is_finite()) {
            return Err("route targets: every curve point must be finite".into());
        }
        if let Some(pair) = self.points.windows(2).find(|pair| pair[0].0 >= pair[1].0) {
            return Err(format!(
                "route targets: curve points must be strictly ascending by value, got {} before {}",
                pair[0].0, pair[1].0
            )
            .into());
        }

        Ok(())
    }
}

const SPANS: usize = 4;

fn span_index(span: RouteCostSpan) -> usize {
    match span {
        RouteCostSpan::DepotToDepot => 0,
        RouteCostSpan::DepotToLastJob => 1,
        RouteCostSpan::FirstJobToDepot => 2,
        RouteCostSpan::FirstJobToLastJob => 3,
    }
}

fn span_of(index: usize) -> RouteCostSpan {
    match index {
        0 => RouteCostSpan::DepotToDepot,
        1 => RouteCostSpan::DepotToLastJob,
        2 => RouteCostSpan::FirstJobToDepot,
        _ => RouteCostSpan::FirstJobToLastJob,
    }
}

fn starts_at_depot(span: RouteCostSpan) -> bool {
    matches!(span, RouteCostSpan::DepotToDepot | RouteCostSpan::DepotToLastJob)
}

fn ends_at_depot(span: RouteCostSpan) -> bool {
    matches!(span, RouteCostSpan::DepotToDepot | RouteCostSpan::FirstJobToDepot)
}

/// What one span of a route measures.
#[derive(Clone, Copy, Debug, Default)]
struct SpanMetrics {
    distance: Float,
    driving_time: Float,
    elapsed: Float,
    break_time: Float,
}

/// The measured metrics of one route, every span at once so any target reads from one cache.
#[derive(Clone, Copy, Debug, Default)]
pub struct RouteMetrics {
    spans: [SpanMetrics; SPANS],
    service_time: Float,
    value: Float,
    activities: Float,
}

impl RouteMetrics {
    /// Working hours: first job to last job, breaks taken out.
    fn hours(&self) -> Float {
        let span = &self.spans[span_index(RouteCostSpan::FirstJobToLastJob)];
        (span.elapsed - span.break_time).max(0.) / 3600.
    }

    /// The target's reading, `None` when its divisor is zero.
    fn get(&self, target: &RouteTarget) -> Option<Float> {
        let span = &self.spans[span_index(target.span)];
        let total = match target.metric {
            RouteMetric::Distance => span.distance,
            RouteMetric::DrivingTime => span.driving_time,
            RouteMetric::Duration => (span.elapsed - if target.include_breaks { 0. } else { span.break_time }).max(0.),
            RouteMetric::ServiceTime => self.service_time,
            RouteMetric::Value => self.value,
            RouteMetric::Activities => self.activities,
        };

        let divisor = match target.per {
            RouteTargetPer::Route => return Some(total),
            RouteTargetPer::Activity => self.activities,
            RouteTargetPer::Hour => self.hours(),
        };

        (divisor > 0.).then(|| total / divisor)
    }
}

type JobValueFn = Arc<dyn Fn(&Job) -> Float + Send + Sync>;
type IsBreakFn = Arc<dyn Fn(&Single) -> bool + Send + Sync>;

/// Builds the route targets feature.
pub struct RouteTargetsBuilder {
    name: String,
    transport: Option<Arc<dyn TransportCost + Send + Sync>>,
    targets: Vec<RouteTarget>,
    aggregate: RouteTargetAggregate,
    job_value_fn: Option<JobValueFn>,
    is_break_fn: Option<IsBreakFn>,
}

impl RouteTargetsBuilder {
    /// Creates a new instance of `RouteTargetsBuilder`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            transport: None,
            targets: Vec::new(),
            aggregate: RouteTargetAggregate::SumOfSquares,
            job_value_fn: None,
            is_break_fn: None,
        }
    }

    /// Sets the transport the distances and driving times are read from.
    pub fn set_transport(mut self, transport: Arc<dyn TransportCost + Send + Sync>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Sets the targets.
    pub fn set_targets(mut self, targets: Vec<RouteTarget>) -> Self {
        self.targets = targets;
        self
    }

    /// Sets how a route's points combine. Default is [`RouteTargetAggregate::SumOfSquares`].
    pub fn set_aggregate(mut self, aggregate: RouteTargetAggregate) -> Self {
        self.aggregate = aggregate;
        self
    }

    /// Sets the value of a job, for [`RouteMetric::Value`]. Default is zero for every job.
    pub fn set_job_value_fn<F: Fn(&Job) -> Float + Send + Sync + 'static>(mut self, func: F) -> Self {
        self.job_value_fn = Some(Arc::new(func));
        self
    }

    /// Sets how a break is told from a job. Default is no activity being a break.
    pub fn set_is_break_fn<F: Fn(&Single) -> bool + Send + Sync + 'static>(mut self, func: F) -> Self {
        self.is_break_fn = Some(Arc::new(func));
        self
    }

    /// Builds the feature.
    pub fn build(mut self) -> GenericResult<Feature> {
        let transport = self.transport.take().ok_or_else(|| GenericError::from("route targets: transport required"))?;
        if self.targets.is_empty() {
            return Err("route targets: at least one target required".into());
        }
        self.targets.iter().try_for_each(RouteTarget::validate)?;

        let shared = Arc::new(RouteTargetsShared {
            transport,
            targets: self.targets,
            aggregate: self.aggregate,
            job_value_fn: self.job_value_fn.take().unwrap_or_else(|| Arc::new(|_| 0.)),
            is_break_fn: self.is_break_fn.take().unwrap_or_else(|| Arc::new(|_| false)),
        });

        FeatureBuilder::default()
            .with_name(self.name.as_str())
            .with_objective(RouteTargetsObjective { shared: shared.clone() })
            .with_state(RouteTargetsState { shared })
            .build()
    }
}

struct RouteTargetsShared {
    transport: Arc<dyn TransportCost + Send + Sync>,
    targets: Vec<RouteTarget>,
    aggregate: RouteTargetAggregate,
    job_value_fn: JobValueFn,
    is_break_fn: IsBreakFn,
}

/// The curve read at `x`: linear between points, the outer slope beyond them, never below zero.
fn curve(points: &[(Float, Float)], x: Float) -> Float {
    let last = points.len() - 1;
    let segment = points.windows(2).position(|w| x <= w[1].0).unwrap_or(last - 1);
    let ((x0, y0), (x1, y1)) = (points[segment], points[segment + 1]);
    let slope = if (x1 - x0).abs() > 1e-9 { (y1 - y0) / (x1 - x0) } else { 0. };

    (y0 + slope * (x - x0)).max(0.)
}

impl RouteTargetsShared {
    fn is_break(&self, activity: &Activity) -> bool {
        activity.job.as_ref().is_some_and(|single| (self.is_break_fn)(single))
    }

    fn measure(&self, route: &Route) -> RouteMetrics {
        let profile = &route.actor.vehicle.profile;
        let all: Vec<&Activity> = route.tour.all_activities().collect();
        let work: Vec<&&Activity> = all.iter().filter(|a| a.job.is_some() && !self.is_break(a)).collect();
        if work.is_empty() {
            return RouteMetrics::default();
        }
        let (Some(first), Some(last)) =
            (all.iter().position(|a| a.job.is_some()), all.iter().rposition(|a| a.job.is_some()))
        else {
            return RouteMetrics::default();
        };
        let end = all.len() - 1;

        let spans = std::array::from_fn(|index| {
            let span = span_of(index);
            let from = if starts_at_depot(span) { 0 } else { first };
            let to = if ends_at_depot(span) { end } else { last };
            let activities = &all[from..=to];

            let (distance, driving_time) = activities.windows(2).fold((0., 0.), |(distance, driving), w| {
                let (a, b) = (w[0].place.location, w[1].place.location);
                (
                    distance + self.transport.distance_approx(profile, a, b),
                    driving + self.transport.duration_approx(profile, a, b),
                )
            });
            // A job is left at its departure and reached at its arrival; a depot the other way round.
            let start_time =
                if all[from].job.is_some() { all[from].schedule.arrival } else { all[from].schedule.departure };
            let end_time = if all[to].job.is_some() { all[to].schedule.departure } else { all[to].schedule.arrival };
            let break_time = activities.iter().filter(|a| self.is_break(a)).map(|a| a.place.duration).sum();

            SpanMetrics { distance, driving_time, elapsed: (end_time - start_time).max(0.), break_time }
        });

        let service_time = work.iter().map(|a| a.place.duration).sum();
        let value = route
            .tour
            .jobs()
            .filter(|job| job.as_single().is_none_or(|s| !(self.is_break_fn)(s)))
            .map(|job| (self.job_value_fn)(job))
            .sum();

        RouteMetrics { spans, service_time, value, activities: work.len() as Float }
    }

    fn score(&self, metrics: &RouteMetrics) -> Cost {
        if metrics.activities <= 0. {
            return 0.;
        }
        let points = self.targets.iter().map(|target| metrics.get(target).map_or(0., |x| curve(&target.points, x)));

        match self.aggregate {
            RouteTargetAggregate::Sum => points.sum(),
            RouteTargetAggregate::SumOfSquares => points.map(|points| points * points).sum(),
            RouteTargetAggregate::Max => points.fold(0., Float::max),
        }
    }

    fn metrics(&self, route_ctx: &RouteContext) -> RouteMetrics {
        route_ctx.state().get_route_target_metrics().copied().unwrap_or_else(|| self.measure(route_ctx.route()))
    }

    /// The position-free part of an insertion: the job's value, its count and its service time.
    fn with_job(&self, metrics: &RouteMetrics, job: &Job) -> RouteMetrics {
        let singles: Vec<&Single> = match job {
            Job::Single(single) => vec![single.as_ref()],
            Job::Multi(multi) => multi.jobs.iter().map(|s| s.as_ref()).collect(),
        };
        let singles: Vec<&Single> = singles.into_iter().filter(|s| !(self.is_break_fn)(s)).collect();
        if singles.is_empty() {
            return *metrics;
        }
        let service: Float = singles.iter().filter_map(|s| s.places.first().map(|p| p.duration)).sum();

        let mut spans = metrics.spans;
        spans.iter_mut().for_each(|span| span.elapsed += service);

        RouteMetrics {
            spans,
            service_time: metrics.service_time + service,
            value: metrics.value + (self.job_value_fn)(job),
            activities: metrics.activities + singles.len() as Float,
        }
    }
}

struct RouteTargetsObjective {
    shared: Arc<RouteTargetsShared>,
}

impl FeatureObjective for RouteTargetsObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution.solution.state.get_route_targets_fitness().copied().unwrap_or_else(|| {
            solution.solution.routes.iter().map(|route_ctx| self.shared.score(&self.shared.metrics(route_ctx))).sum()
        })
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            // Priced per route as far as the position does not matter: the evaluator skips a route
            // whose route estimate already loses, so a reward for filling a short day has to be seen
            // here, not only at the position.
            MoveContext::Route { route_ctx, job, .. } => {
                let old = self.shared.metrics(route_ctx);
                self.shared.score(&self.shared.with_job(&old, job)) - self.shared.score(&old)
            }
            // What the position adds on top: the detour inside each span.
            MoveContext::Activity { route_ctx, activity_ctx, .. } => {
                let Some(single) = activity_ctx.target.job.as_ref() else { return Cost::default() };
                if (self.shared.is_break_fn)(single) {
                    return Cost::default();
                }
                let profile = &route_ctx.route().actor.vehicle.profile;
                let transport = &self.shared.transport;
                let leg = |from: Location, to: Location| {
                    (transport.distance_approx(profile, from, to), transport.duration_approx(profile, from, to))
                };

                let old = self.shared.metrics(route_ctx);
                let is_empty = old.activities <= 0.;
                let base = self.shared.with_job(&old, &Job::Single(single.clone()));
                let (prev, target, next) = (activity_ctx.prev, activity_ctx.target.place.location, activity_ctx.next);

                let mut moved = base;
                moved.spans.iter_mut().enumerate().for_each(|(index, span)| {
                    let kind = span_of(index);
                    // The new job always lies inside the span, so a leg to it counts once its other
                    // end does: a job, or a depot the span reaches.
                    let prev_inside = prev.job.is_some() || starts_at_depot(kind);
                    let next_inside = next.is_some_and(|n| n.job.is_some() || ends_at_depot(kind));

                    let (mut distance, mut duration) = (0., 0.);
                    if prev_inside {
                        let (d, t) = leg(prev.place.location, target);
                        (distance, duration) = (distance + d, duration + t);
                    }
                    if let Some(next) = next.filter(|_| next_inside) {
                        let (d, t) = leg(target, next.place.location);
                        (distance, duration) = (distance + d, duration + t);
                        if prev_inside && !is_empty {
                            let (d, t) = leg(prev.place.location, next.place.location);
                            (distance, duration) = (distance - d, duration - t);
                        }
                    }

                    span.distance += distance.max(0.);
                    span.driving_time += duration.max(0.);
                    span.elapsed += duration.max(0.);
                });

                self.shared.score(&moved) - self.shared.score(&base)
            }
        }
    }
}

struct RouteTargetsState {
    shared: Arc<RouteTargetsShared>,
}

impl FeatureState for RouteTargetsState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, _: &Job) {
        let route_ctx = solution_ctx.routes.get_mut(route_index).expect("route_index out of bounds");
        self.accept_route_state(route_ctx);
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        let metrics = self.shared.measure(route_ctx.route());
        route_ctx.state_mut().set_route_target_metrics(metrics);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx.routes.iter_mut().for_each(|route_ctx| self.accept_route_state(route_ctx));
        let fitness =
            solution_ctx.routes.iter().map(|route_ctx| self.shared.score(&self.shared.metrics(route_ctx))).sum();
        solution_ctx.state.set_route_targets_fitness(fitness);
    }
}
