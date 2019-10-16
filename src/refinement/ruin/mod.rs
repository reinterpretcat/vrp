use crate::construction::states::*;
use crate::models::problem::Job;
use crate::models::solution::Route;
use crate::models::{Problem, Solution};
use crate::refinement::RefinementContext;
use crate::utils::{DefaultRandom, Random};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Specifies ruin strategy.
pub trait RuinStrategy {
    fn ruin_solution(&self, refinement_ctx: &RefinementContext) -> Result<InsertionContext, String>;
}

/// Creates insertion context from existing solution.
fn create_insertion_context(
    problem: &Arc<Problem>,
    individuum: &(Arc<Solution>, ObjectiveCost),
    random: &Arc<dyn Random + Send + Sync>,
) -> InsertionContext {
    let (solution, cost) = individuum;
    let jobs: Vec<Arc<Job>> = solution.unassigned.iter().map(|(job, _)| job.clone()).collect();
    let mut registry = solution.registry.deep_copy();
    let mut routes: HashSet<RouteContext> = HashSet::new();

    solution.routes.iter().for_each(|route| {
        if route.tour.has_jobs() {
            let mut route_ctx = RouteContext {
                route: Arc::new(RwLock::new(route.deep_copy())),
                state: Arc::new(RwLock::new(RouteState::new())),
            };
            problem.constraint.accept_route_state(&mut route_ctx);
            routes.insert(route_ctx);
        } else {
            registry.free_actor(&route.actor);
        }
    });

    InsertionContext {
        progress: InsertionProgress {
            cost: Some(cost.total()),
            completeness: 1. - (solution.unassigned.len() as f64 / problem.jobs.size() as f64),
            total: problem.jobs.size(),
        },
        problem: problem.clone(),
        solution: SolutionContext { required: jobs, ignored: vec![], unassigned: Default::default(), routes, registry },
        random: random.clone(),
    }
}

mod adjusted_string_removal;

pub use self::adjusted_string_removal::AdjustedStringRemoval;
use crate::models::common::ObjectiveCost;

mod random_route_removal;
