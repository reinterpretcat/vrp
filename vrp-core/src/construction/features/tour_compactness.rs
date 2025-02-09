#[cfg(test)]
#[path = "../../../tests/unit/construction/features/tour_compactness_test.rs"]
mod tour_compactness_test;

use super::*;
use crate::construction::enablers::FeatureCombinator;
use crate::models::solution::Activity;
use crate::utils::Either;

/// Creates a feature which tries to keep tours (routes) compact
///
/// `cost_feature` A feature to be used as a main cost.
/// `num_representative_points`: Number of representative points to be used route distance calculation.
/// `distance_fn`: A distance/cost function.
pub fn create_tour_compactness_feature<F>(
    cost_feature: Feature,
    num_representative_points: usize,
    distance_fn: F,
) -> Result<Feature, GenericError>
where
    F: Fn(&Actor, Location, Location) -> Cost + Send + Sync + 'static,
{
    if cost_feature.objective.is_none() {
        return Err(GenericError::from("tour compactness requires cost feature to have an objective"));
    }

    if num_representative_points < 2 {
        return Err(GenericError::from("tour compactness requires at least 2 representative points"));
    }

    // use feature combinator to inject a different objective
    FeatureCombinator::default()
        .use_name(cost_feature.name.as_str())
        .add_feature(cost_feature)
        .set_objective_combinator(move |objectives| {
            if objectives.len() != 1 {
                return Err(GenericError::from("tour compactness feature requires exactly one cost objective"));
            }

            let objective = objectives[0].1.clone();

            Ok(Some(Arc::new(TourCompactnessObjective { objective, num_representative_points, distance_fn })))
        })
        .combine()
}

struct TourCompactnessObjective<F> {
    objective: Arc<dyn FeatureObjective>,
    num_representative_points: usize,
    distance_fn: F,
}

impl<F> FeatureObjective for TourCompactnessObjective<F>
where
    F: Fn(&Actor, Location, Location) -> Cost + Send + Sync,
{
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        self.objective.fitness(solution)
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { solution_ctx, route_ctx, job } => {
                if route_ctx.route().tour.has_jobs() {
                    return self.objective.estimate(move_ctx);
                }

                job.as_single()
                    .iter()
                    // NOTE we can calculate dispersion bonus for a single job with no alternative places
                    // and skip calculation in activity evaluation as it is not going to change
                    .filter(|single| single.places.len() == 1)
                    .filter_map(|single| single.places.first())
                    .filter_map(|place| place.location)
                    .map(|location| {
                        -self.calculate_dispersion_bonus(solution_ctx.routes.as_slice(), route_ctx, location)
                    })
                    .next()
                    .unwrap_or_default()
            }
            MoveContext::Activity { solution_ctx, route_ctx, activity_ctx } => {
                if route_ctx.route().tour.has_jobs() {
                    return self.objective.estimate(move_ctx);
                }

                // check if we have already considered dispersion bonus for the job
                let dispersion_bonus = if activity_ctx
                    .target
                    .retrieve_job()
                    .map(|job| job.as_single().is_some_and(|single| single.places.len() == 1))
                    .unwrap_or(false)
                {
                    Cost::default()
                } else {
                    self.calculate_dispersion_bonus(
                        solution_ctx.routes.as_slice(),
                        route_ctx,
                        activity_ctx.target.place.location,
                    )
                };

                self.objective.estimate(move_ctx) - dispersion_bonus
            }
        }
    }
}

impl<F> TourCompactnessObjective<F>
where
    F: Fn(&Actor, Location, Location) -> Cost + Send + Sync,
{
    /// Calculates dispersion bonus for the location. A general idea is to reward the assignment of
    /// the next activity in the **new** tour if it spawns it far from the existing tours.
    fn calculate_dispersion_bonus(
        &self,
        routes: &[RouteContext],
        current_route: &RouteContext,
        location: Location,
    ) -> Cost {
        if routes.len() == 1 {
            return Cost::default();
        }
        let (min_distances, max_loads) = routes.iter().fold((0., 0.), |(min_distances, max_loads), route_ctx| {
            let distance = if route_ctx == current_route {
                Cost::default()
            } else {
                self.calculate_distance(current_route, route_ctx, location)
            };

            (
                min_distances + distance,
                max_loads + route_ctx.state().get_max_vehicle_load().copied().unwrap_or_default(),
            )
        });

        let utilization_ratio = max_loads / (routes.len() as f64 - 1.);
        let avg_min_distances = min_distances / (routes.len() as f64 - 1.);

        // Utilization ratio is used to decrease the bonus to reduce likeness of new route assignment
        // Quadratic-Exponential Blend: amplify bonus when utilization is high
        let (k, p) = (0.5, 8.);
        let utilization_weight = utilization_ratio.powf(p) + (1. - (-k * utilization_ratio).exp());

        avg_min_distances * utilization_weight
    }

    /// Calculates distance between location from `this_route` and representative activities from `other_route`.
    fn calculate_distance(&self, this_route: &RouteContext, other_route: &RouteContext, from: Location) -> Cost {
        let with_start = this_route
            .route()
            .tour
            .start()
            .zip(other_route.route().tour.start())
            .map(|(this, other)| this.place.location != other.place.location)
            .unwrap_or(true);
        // TODO: consider closed vs open VRP use cases
        let with_end = this_route
            .route()
            .tour
            .end()
            .zip(other_route.route().tour.end())
            .map(|(this, other)| this.place.location != other.place.location)
            .unwrap_or(true);

        self.get_representative_activities(other_route, with_start, with_end)
            .map(|activity| (self.distance_fn)(&this_route.route().actor, from, activity.place.location))
            .min_by(|a, b| a.total_cmp(b))
            .unwrap_or_default()
    }

    /// Returns representative activities from the route.
    fn get_representative_activities<'a>(
        &self,
        route_ctx: &'a RouteContext,
        with_start: bool,
        with_end: bool,
    ) -> impl Iterator<Item = &'a Activity> + 'a {
        debug_assert!(self.num_representative_points >= 2);

        let tour = &route_ctx.route().tour;
        let total = tour.total();

        if total < self.num_representative_points {
            return Either::Left(tour.all_activities());
        }

        let start = usize::from(!with_start);
        let end = total.saturating_sub(1 + usize::from(!with_end));

        let segment_length = end.saturating_sub(start + 1);
        let samples = self.num_representative_points.min(total);
        let step = segment_length as f64 / samples as f64;

        let (start, end) = if step == 0. { (0, total.saturating_sub(1)) } else { (start, end) };

        Either::Right(
            (1..=samples)
                .filter_map(move |i| match i {
                    1 => Some(start),
                    n if n == samples => Some(end),
                    i => {
                        let idx = (start as f64 + 1.0 + (i - 1) as f64 * step).round() as usize;
                        (idx < end).then_some(idx)
                    }
                })
                .filter_map(|idx| tour.get(idx)),
        )
    }
}
