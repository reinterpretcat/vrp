#[cfg(test)]
#[path = "../../../tests/unit/construction/features/tour_compactness_test.rs"]
mod tour_compactness_test;

use super::*;

custom_solution_state!(TourCompactness typeof Cost);

/// Creates a feature which tries to keep routes compact by reducing amount of jobs in their
/// neighbourhood served by different routes.
///
/// `job_radius` controls amount of jobs checked in neighbourhood of a tested one.
pub fn create_tour_compactness_feature(
    name: &str,
    jobs: Arc<Jobs>,
    job_radius: usize,
) -> Result<Feature, GenericError> {
    if job_radius < 1 {
        return Err("Tour compactness: job radius should be at least 1".into());
    }

    // Reference scale for self-normalization: the theoretical maximum fitness, reached when every
    // job has all `job_radius` of its nearest neighbours served by foreign routes. The per-job
    // counts sum to at most `N * job_radius`, halved like the fitness itself (each shared edge is
    // counted from both endpoints). Dividing fitness by this maps it to a dimensionless [0, 1]
    // "fraction of neighbourhoods split across routes", so weights in a scalarizing multi-objective
    // become comparable across problems of different size. Guarded to stay positive for tiny inputs.
    let fitness_scale = (jobs.size() as Cost * job_radius as Cost / 2.).max(1.);

    FeatureBuilder::default()
        .with_name(name)
        .with_objective(TourCompactnessObjective { jobs: jobs.clone(), job_radius, fitness_scale })
        .with_state(TourCompactnessState { jobs, job_radius })
        .build()
}

struct TourCompactnessObjective {
    jobs: Arc<Jobs>,
    job_radius: usize,
    fitness_scale: Cost,
}

impl FeatureObjective for TourCompactnessObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
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

    fn fitness_scale(&self) -> Cost {
        self.fitness_scale
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
