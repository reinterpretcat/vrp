#[cfg(test)]
#[path = "../../../tests/unit/construction/features/co_located_test.rs"]
mod co_located_test;

use super::*;
use rustc_hash::FxHashMap;
use std::collections::HashMap;

custom_solution_state!(SplitCoLocatedPairs typeof Cost);

/// Creates a feature which counts co-located pairs split across tours: pairs whose two jobs are
/// both assigned, but to different tours. A pair with an unassigned job counts nothing.
///
/// Each pair counts once, however often and in whichever order it is given, so the fitness is a
/// plain number of pairs and the estimate of inserting a job is the number of its partners already
/// served by another tour.
///
/// Partners are looked up in the routes a solution holds, so a sub-problem holding only some of the
/// routes counts what it sees instead of refusing to run.
pub fn create_co_located_feature(name: &str, pairs: Vec<(Job, Job)>) -> Result<Feature, GenericError> {
    let partners = pairs.into_iter().fold(HashMap::<Job, Vec<Job>>::new(), |mut acc, (a, b)| {
        if !acc.get(&a).is_some_and(|partners| partners.contains(&b)) {
            acc.entry(a.clone()).or_default().push(b.clone());
            acc.entry(b).or_default().push(a);
        }
        acc
    });

    // Reference scale for a scalarizing multi-objective: the fitness when every pair is split.
    let fitness_scale = (partners.values().map(Vec::len).sum::<usize>() as Cost / 2.).max(1.);
    // Counted after every search step: each job of a pair gets a slot, so a count fills one slot per
    // assigned job and compares the pairs, instead of hashing every job of the solution into a map.
    let slots = partners.keys().enumerate().map(|(slot, job)| (job.clone(), slot)).collect::<FxHashMap<_, _>>();
    let pairs = partners
        .iter()
        .flat_map(|(job, partners)| partners.iter().map(move |partner| (job, partner)))
        .map(|(job, partner)| (slots[job], slots[partner]))
        .filter(|(slot, partner)| slot < partner)
        .collect();
    let partners = Arc::new(partners);

    FeatureBuilder::default()
        .with_name(name)
        .with_objective(CoLocatedObjective { partners, fitness_scale })
        .with_state(CoLocatedState { slots, pairs })
        .build()
}

struct CoLocatedObjective {
    partners: Arc<HashMap<Job, Vec<Job>>>,
    fitness_scale: Cost,
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

    fn fitness_scale(&self) -> Cost {
        self.fitness_scale
    }
}

struct CoLocatedState {
    /// The slot of every job that has a partner.
    slots: FxHashMap<Job, usize>,
    /// Every pair once, by slot. A job paired with itself is left out: it can never be split.
    pairs: Vec<(usize, usize)>,
}

impl FeatureState for CoLocatedState {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        let mut route_of = vec![usize::MAX; self.slots.len()];
        solution_ctx.routes.iter().enumerate().for_each(|(idx, route_ctx)| {
            route_ctx.route().tour.jobs().filter_map(|job| self.slots.get(job)).for_each(|&slot| route_of[slot] = idx);
        });

        let split = self
            .pairs
            .iter()
            .filter(|&&(a, b)| route_of[a] != usize::MAX && route_of[b] != usize::MAX && route_of[a] != route_of[b])
            .count();

        solution_ctx.state.set_split_co_located_pairs(split as Cost);
    }
}
