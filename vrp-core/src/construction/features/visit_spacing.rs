//! A feature keeping the visits of one rhythm apart by their interval, give or take a tolerance.

use super::*;
use std::collections::HashMap;

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/visit_spacing_test.rs"]
mod visit_spacing_test;

/// Where a visit sits in its rhythm and how far it keeps from the others.
#[derive(Clone, Debug)]
pub struct VisitSpacing {
    /// The rhythm the visit belongs to, e.g. one subscription.
    pub group: String,
    /// The visit's place in the rhythm: 1 comes before 2.
    pub order: usize,
    /// Days between two consecutive visits.
    pub interval_days: i64,
    /// Days a visit may land off its interval, per step of the rhythm.
    pub tolerance_days: i64,
    /// An anchor is never refused: its day is a fact, and the others keep their distance to it.
    pub anchor: bool,
    /// The visit's offset in days within the rhythm. Where two visits both carry one, they lie at least the
    /// difference of their offsets apart, with no upper bound, instead of keeping an interval.
    pub min_offset_days: Option<i64>,
}

custom_dimension!(pub VisitSpacing typeof VisitSpacing);
custom_tour_state!(CurrentSpacedVisits typeof HashMap<String, Vec<(usize, Option<i64>)>>);

const SECONDS_PER_DAY: Float = 86_400.;

/// Creates the visit spacing feature as a hard constraint.
///
/// A visit is refused on a day too close to, or too far from, the nearest visit of its rhythm
/// already placed before or after it: `steps` apart in order they must lie `steps * interval`
/// days apart, give or take `steps * tolerance`. The tolerance grows with the steps so that
/// taking out the visit between two others never leaves those two apart by more than they may.
///
/// The day of a visit is its shift's day: two shifts are whole days apart however their start
/// times differ by the hour, so the distance is rounded to days.
pub fn create_visit_spacing_feature(
    name: &str,
    total_jobs: usize,
    code: ViolationCode,
) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_constraint(VisitSpacingConstraint { total_jobs, code })
        .with_state(VisitSpacingState {})
        .build()
}

struct VisitSpacingConstraint {
    total_jobs: usize,
    code: ViolationCode,
}

impl FeatureConstraint for VisitSpacingConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { solution_ctx, route_ctx, job } => {
                let spacing = job.dimens().get_visit_spacing().filter(|spacing| !spacing.anchor)?;

                // A sub-problem sees only some of the routes, so it cannot tell where the rest of
                // the rhythm lies: refuse, as the group features do.
                if solution_ctx.get_jobs_amount() != self.total_jobs {
                    return ConstraintViolation::fail(self.code);
                }

                let day = route_start(route_ctx);
                let mut before: Option<(usize, Option<i64>, Float)> = None;
                let mut after: Option<(usize, Option<i64>, Float)> = None;

                solution_ctx.routes.iter().for_each(|rc| {
                    let Some(orders) =
                        rc.state().get_current_spaced_visits().and_then(|visits| visits.get(&spacing.group))
                    else {
                        return;
                    };
                    let other_day = route_start(rc);

                    orders.iter().for_each(|&(order, offset)| {
                        if order < spacing.order && before.is_none_or(|(nearest, _, _)| order > nearest) {
                            before = Some((order, offset, other_day));
                        }
                        if order > spacing.order && after.is_none_or(|(nearest, _, _)| order < nearest) {
                            after = Some((order, offset, other_day));
                        }
                    });
                });

                let own = (spacing.order, spacing.min_offset_days, day);
                let keeps_apart = |other: (usize, Option<i64>, Float)| {
                    is_spaced(spacing, other, own) && is_spaced(spacing, own, other)
                };

                if before.is_none_or(keeps_apart) && after.is_none_or(keeps_apart) {
                    None
                } else {
                    ConstraintViolation::fail(self.code)
                }
            }
            MoveContext::Activity { .. } => None,
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        match (source.dimens().get_visit_spacing(), candidate.dimens().get_visit_spacing()) {
            (None, None) => Ok(source),
            _ => Err(self.code),
        }
    }
}

/// Whether the `later` visit keeps its distance to the `earlier` one, each given as its order, offset and day.
/// Only the pair whose orders run forward is measured; the other one passes. Two visits with offsets lie at
/// least the difference of their offsets apart; otherwise they keep the interval, give or take the tolerance.
fn is_spaced(spacing: &VisitSpacing, earlier: (usize, Option<i64>, Float), later: (usize, Option<i64>, Float)) -> bool {
    let ((earlier_order, earlier_offset, earlier_day), (later_order, later_offset, later_day)) = (earlier, later);

    if later_order <= earlier_order {
        return true;
    }

    let days = ((later_day - earlier_day) / SECONDS_PER_DAY).round() as i64;

    if let (Some(earlier_offset), Some(later_offset)) = (earlier_offset, later_offset) {
        return days >= later_offset - earlier_offset;
    }

    let steps = (later_order - earlier_order) as i64;

    (days - steps * spacing.interval_days).abs() <= steps * spacing.tolerance_days
}

fn route_start(route_ctx: &RouteContext) -> Float {
    route_ctx.route().actor.detail.time.start
}

struct VisitSpacingState {}

impl FeatureState for VisitSpacingState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        if job.dimens().get_visit_spacing().is_some() {
            let route_ctx = solution_ctx.routes.get_mut(route_index).unwrap();
            let visits = collect_spaced_visits(route_ctx);
            route_ctx.state_mut().set_current_spaced_visits(visits);
        }
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        let visits = collect_spaced_visits(route_ctx);
        route_ctx.state_mut().set_current_spaced_visits(visits);
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx.routes.iter_mut().for_each(|route_ctx| {
            let visits = collect_spaced_visits(route_ctx);
            route_ctx.state_mut().set_current_spaced_visits(visits);
        });
    }
}

fn collect_spaced_visits(route_ctx: &RouteContext) -> HashMap<String, Vec<(usize, Option<i64>)>> {
    route_ctx.route().tour.jobs().filter_map(|job| job.dimens().get_visit_spacing()).fold(
        HashMap::default(),
        |mut visits, spacing| {
            visits.entry(spacing.group.clone()).or_default().push((spacing.order, spacing.min_offset_days));
            visits
        },
    )
}
