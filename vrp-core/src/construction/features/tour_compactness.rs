#[cfg(test)]
#[path = "../../../tests/unit/construction/features/tour_compactness_test.rs"]
mod tour_compactness_test;

use super::*;
use std::cmp::Ordering;

custom_solution_state!(TourCompactness typeof Cost);

/// Creates a feature which tries to keep routes compact by reducing amount of jobs in their
/// neighbourhood served by different routes.
///
/// `job_radius` controls amount of jobs checked in neighbourhood of a tested one.
///
/// `thresholds` parameter specifies (`min_threshold`, `min_distance`) pair of thresholds used to relax
/// objective in favor of less prioritized ones.
pub fn create_tour_compactness_feature(
    name: &str,
    jobs: Arc<Jobs>,
    job_radius: usize,
    thresholds: Option<(usize, f64)>,
) -> Result<Feature, GenericError> {
    if job_radius < 1 {
        return Err("Tour compactness: job radius should be at least 1".into());
    }

    let thresholds = thresholds.map(|(threshold, distance)| (threshold as f64, distance));

    FeatureBuilder::default()
        .with_name(name)
        .with_objective(TourCompactnessObjective { jobs: jobs.clone(), job_radius, thresholds })
        .with_state(TourCompactnessState { jobs, job_radius })
        .build()
}

struct TourCompactnessObjective {
    jobs: Arc<Jobs>,
    job_radius: usize,
    thresholds: Option<(f64, f64)>,
}

impl FeatureObjective for TourCompactnessObjective {
    fn total_order(&self, a: &InsertionContext, b: &InsertionContext) -> Ordering {
        let fitness_a = self.fitness(a);
        let fitness_b = self.fitness(b);

        if let Some((min_threshold, min_distance)) = self.thresholds {
            // check first threshold
            match (compare_floats(fitness_a, min_threshold), compare_floats(fitness_b, min_threshold)) {
                (Ordering::Less, Ordering::Less) => return Ordering::Equal,
                (Ordering::Less, _) => return Ordering::Less,
                (_, Ordering::Less) => return Ordering::Greater,
                _ => {}
            };

            // check second difference
            let relative_difference = (fitness_a - fitness_b).abs() / fitness_a.max(fitness_b).abs();
            if relative_difference < min_distance {
                return Ordering::Equal;
            }
        }

        compare_floats(fitness_a, fitness_b)
    }

    fn fitness(&self, solution: &InsertionContext) -> f64 {
        solution.solution.state.get_tour_compactness().copied().unwrap_or_default()
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { solution_ctx, route_ctx, job } => {
                count_shared_neighbours((solution_ctx, route_ctx, job), &self.jobs, self.job_radius) as Cost
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct TourCompactnessState {
    jobs: Arc<Jobs>,
    job_radius: usize,
}

impl FeatureState for TourCompactnessState {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        let fitness = solution_ctx.routes.iter().fold(Cost::default(), |acc, route_ctx| {
            acc + route_ctx
                .route()
                .tour
                .jobs()
                .map(|job| count_shared_neighbours((solution_ctx, route_ctx, job), &self.jobs, self.job_radius))
                .sum::<usize>() as Cost
        }) / 2.;

        solution_ctx.state.set_tour_compactness(fitness);
    }
}

fn count_shared_neighbours(item: (&SolutionContext, &RouteContext, &Job), jobs: &Jobs, job_radius: usize) -> usize {
    let (solution_ctx, route_ctx, job) = item;

    let route = route_ctx.route();
    let departure = route.tour.start().map_or(Timestamp::default(), |s| s.schedule.departure);

    jobs.neighbors(&route.actor.vehicle.profile, job, departure)
        .take(job_radius)
        .filter(|(j, _)| {
            let not_current = !route.tour.has_job(j);
            let is_assigned = !solution_ctx.required.contains(j);

            not_current && is_assigned
        })
        .count()
}
