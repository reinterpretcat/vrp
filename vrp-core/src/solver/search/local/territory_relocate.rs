//! An OPT-IN local-search operator that actively pursues spatial compactness. The default random
//! move generators (`select_seed_job` picks a random route + random job) only stumble on
//! compactness-improving moves, so a soft territory objective is never systematically enforced.
//! This operator instead seeds on the *most spatially-misplaced* assigned jobs — those whose nearest
//! neighbours mostly belong to another driver — and relocates each onto that "home" driver (trying
//! all of that driver's day-tours, taking the best feasible insertion). Move acceptance is unchanged
//! (the composite local search keeps it only if the full goal's `total_order` improves), so it is
//! safe: it never worsens the accepted solution.
//!
//! It is registered in the local-search pool only when the goal actually contains the `territory`
//! objective (`GoalContext::has_objective("territory")`); for every other problem the pool is
//! byte-identical to the stock solver. See the gated registration in `solver::heuristic`.

use crate::construction::heuristics::*;
use crate::models::common::Timestamp;
use crate::models::problem::{Job, VehicleIdDimension};
use crate::models::solution::Route;
use crate::solver::RefinementContext;
use crate::solver::search::{LocalOperator, get_route_jobs};
use rosomaxa::prelude::*;
use std::collections::HashMap;

/// Nearest neighbours that vote for a job's "home" driver.
const NEIGHBORS: usize = 10;
/// Random pick is taken among the top-N most-misplaced jobs, to decorrelate consecutive calls.
const TOP_SEEDS: usize = 12;

/// A per-driver key used to group a driver's several day-tours: its vehicle id.
fn driver_of(route: &Route) -> &str {
    route.actor.vehicle.dimens.get_vehicle_id().map(String::as_str).unwrap_or("")
}

/// Relocates the most spatially-misplaced job onto the driver that owns most of its neighbours.
pub struct TerritoryRelocate;

impl LocalOperator for TerritoryRelocate {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        let problem = &insertion_ctx.problem;
        let random = insertion_ctx.environment.random.clone();
        let route_jobs = get_route_jobs(&insertion_ctx.solution);
        let route_driver: Vec<&str> =
            insertion_ctx.solution.routes.iter().map(|rc| driver_of(rc.route())).collect();

        // Score assigned jobs by how strongly their neighbourhood pulls them onto another driver.
        let mut candidates: Vec<(f64, Job, String)> = Vec::new(); // (score, job, home_driver)
        for (route_idx, route_ctx) in insertion_ctx.solution.routes.iter().enumerate() {
            let own_driver = route_driver[route_idx];
            let profile = route_ctx.route().actor.vehicle.profile.clone();
            for job in route_ctx.route().tour.jobs() {
                if insertion_ctx.solution.locked.contains(job) {
                    continue;
                }
                let mut per_driver: HashMap<&str, usize> = HashMap::new();
                let mut seen = 0usize;
                for (neighbor, _) in problem.jobs.neighbors(&profile, job, Timestamp::default()) {
                    if let Some(&r) = route_jobs.get(neighbor) {
                        *per_driver.entry(route_driver[r]).or_insert(0) += 1;
                        seen += 1;
                        if seen >= NEIGHBORS {
                            break;
                        }
                    }
                }
                if seen == 0 {
                    continue;
                }
                let own = *per_driver.get(own_driver).unwrap_or(&0);
                if let Some((&home, &home_cnt)) =
                    per_driver.iter().filter(|(d, _)| **d != own_driver).max_by_key(|(_, c)| **c)
                    && home_cnt > own
                {
                    candidates.push(((home_cnt - own) as f64 / seen as f64, job.clone(), home.to_string()));
                }
            }
        }
        if candidates.is_empty() {
            return None;
        }
        candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
        let pool = candidates.len().min(TOP_SEEDS);
        let (_, seed_job, home_driver) = candidates[random.uniform_int(0, pool as i32 - 1) as usize].clone();

        // Remove the seed from wherever it currently sits.
        let mut new_ctx = insertion_ctx.deep_copy();
        let from_route = new_ctx.solution.routes.iter().position(|rc| rc.route().tour.contains(&seed_job))?;
        {
            let route_ctx = new_ctx.solution.routes.get_mut(from_route)?;
            if !route_ctx.route_mut().tour.remove(&seed_job) {
                return None;
            }
            new_ctx.problem.goal.accept_route_state(route_ctx);
        }

        // Best feasible insertion across ALL of the home driver's day-tours.
        let leg_selection = LegSelection::Stochastic(random.clone());
        let result_selector = BestResultSelector::default();
        let home_routes: Vec<usize> = new_ctx
            .solution
            .routes
            .iter()
            .enumerate()
            .filter(|(_, rc)| driver_of(rc.route()) == home_driver)
            .map(|(idx, _)| idx)
            .collect();

        let result = {
            let eval_ctx = EvaluationContext {
                goal: &new_ctx.problem.goal,
                job: &seed_job,
                leg_selection: &leg_selection,
                result_selector: &result_selector,
            };
            home_routes.iter().fold(InsertionResult::make_failure(), |best, &hr| {
                match new_ctx.solution.routes.get(hr) {
                    Some(route) => {
                        eval_job_insertion_in_route(&new_ctx, &eval_ctx, route, InsertionPosition::Any, best)
                    }
                    None => best,
                }
            })
        };

        match result {
            InsertionResult::Success(success) => {
                apply_insertion_success(&mut new_ctx, success);
                finalize_insertion_ctx(&mut new_ctx);
                Some(new_ctx)
            }
            InsertionResult::Failure(_) => None,
        }
    }
}
