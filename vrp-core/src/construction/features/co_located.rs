#[cfg(test)]
#[path = "../../../tests/unit/construction/features/co_located_test.rs"]
mod co_located_test;

use super::*;
use std::collections::HashMap;

custom_solution_state!(SplitCoLocatedPairs typeof Cost);

/// Creates a feature which counts co-located pairs split across tours: pairs whose two jobs are
/// both assigned, but to different tours. A pair with an unassigned job counts nothing.
///
/// Each pair counts once, so the fitness is a plain number of pairs and the estimate of inserting a
/// job is the number of its partners already served by another tour.
///
/// Partners are looked up in the routes a solution holds, so a sub-problem holding only some of the
/// routes counts what it sees instead of refusing to run.
pub fn create_co_located_feature(name: &str, pairs: Vec<(Job, Job)>) -> Result<Feature, GenericError> {
    let partners = pairs.iter().fold(HashMap::<Job, Vec<Job>>::new(), |mut acc, (a, b)| {
        acc.entry(a.clone()).or_default().push(b.clone());
        acc.entry(b.clone()).or_default().push(a.clone());
        acc
    });

    FeatureBuilder::default()
        .with_name(name)
        .with_objective(CoLocatedObjective { partners: Arc::new(partners) })
        .with_state(CoLocatedState { pairs: Arc::new(pairs) })
        .build()
}

struct CoLocatedObjective {
    partners: Arc<HashMap<Job, Vec<Job>>>,
}

impl FeatureObjective for CoLocatedObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution.solution.state.get_split_co_located_pairs().copied().unwrap_or_default()
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { solution_ctx, route_ctx, job } => {
                let Some(partners) = self.partners.get(job) else { return Cost::default() };
                let tour = &route_ctx.route().tour;

                partners
                    .iter()
                    .filter(|partner| !tour.has_job(partner))
                    .filter(|partner| solution_ctx.routes.iter().any(|other| other.route().tour.has_job(partner)))
                    .count() as Cost
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}

struct CoLocatedState {
    pairs: Arc<Vec<(Job, Job)>>,
}

impl FeatureState for CoLocatedState {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        let tour_of = solution_ctx
            .routes
            .iter()
            .enumerate()
            .flat_map(|(idx, route_ctx)| route_ctx.route().tour.jobs().map(move |job| (job, idx)))
            .collect::<HashMap<_, _>>();

        let split = self
            .pairs
            .iter()
            .filter(|(a, b)| matches!((tour_of.get(a), tour_of.get(b)), (Some(a), Some(b)) if a != b))
            .count();

        solution_ctx.state.set_split_co_located_pairs(split as Cost);
    }
}
